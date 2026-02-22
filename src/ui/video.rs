use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{Duration, Instant},
};

use ffmpeg_next as ffmpeg;
use iced::widget::image;

// iced_wgpu uploads raster images larger than 2 MB asynchronously.
// For per-frame video handles, that can produce visible flicker.
// Keep decoded frame uploads under that limit for stable playback.
const MAX_FRAME_WIDTH: u32 = 960;
const MAX_FRAME_HEIGHT: u32 = 540;

pub struct VideoPlayer {
    path: PathBuf,
    input: ffmpeg::format::context::Input,
    stream_index: usize,
    stream_time_base: ffmpeg::Rational,
    stream_start_us: i64,
    duration_us: Option<i64>,
    decoder: ffmpeg::decoder::Video,
    scaler: ffmpeg::software::scaling::context::Context,
    rotation: Rotation,
    frame_interval: Duration,
    last_tick: Instant,
    accumulator: Duration,
    playing: bool,
    finished: bool,
    position_us: i64,
    current_frame: Option<image::Handle>,
    pending_packet: Option<ffmpeg::Packet>,
    eof_sent: bool,
}

impl VideoPlayer {
    pub fn open(path: &Path) -> Result<Self, String> {
        ensure_ffmpeg_initialized()?;

        let input = ffmpeg::format::input(path)
            .map_err(|err| format!("failed to open video input {}: {err}", path.display()))?;

        let stream = input
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or_else(|| format!("no video stream in {}", path.display()))?;

        let stream_index = stream.index();
        let rotation = rotation_from_stream(&stream);
        let stream_avg_rate = stream.avg_frame_rate();
        let stream_rate = stream.rate();
        let stream_time_base = stream.time_base();
        let stream_start_us = read_stream_start_us(&stream).unwrap_or(0);
        let stream_duration_us = read_stream_duration_us(&stream);
        let container_duration_us = read_container_duration_us(&input);

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
            .map_err(|err| format!("decoder context init failed for {}: {err}", path.display()))?;

        let decoder = context_decoder
            .decoder()
            .video()
            .map_err(|err| format!("video decoder open failed for {}: {err}", path.display()))?;

        let (scaled_width, scaled_height) =
            scaled_output_size(decoder.width(), decoder.height(), rotation);

        let scaler = ffmpeg::software::scaling::context::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGBA,
            scaled_width,
            scaled_height,
            ffmpeg::software::scaling::flag::Flags::BILINEAR,
        )
        .map_err(|err| format!("scaler init failed for {}: {err}", path.display()))?;

        let fps = rational_to_fps(stream_avg_rate)
            .or_else(|| rational_to_fps(stream_rate))
            .or_else(|| decoder.frame_rate().and_then(rational_to_fps))
            .unwrap_or(30.0)
            .clamp(12.0, 60.0);

        let mut player = Self {
            path: path.to_path_buf(),
            input,
            stream_index,
            stream_time_base,
            stream_start_us,
            duration_us: stream_duration_us.or(container_duration_us),
            decoder,
            scaler,
            rotation,
            frame_interval: Duration::from_secs_f32(1.0 / fps),
            last_tick: Instant::now(),
            accumulator: Duration::ZERO,
            playing: true,
            finished: false,
            position_us: 0,
            current_frame: None,
            pending_packet: None,
            eof_sent: false,
        };

        let _ = player.decode_next_frame();

        Ok(player)
    }

    pub fn frame_handle(&self) -> Option<image::Handle> {
        self.current_frame.clone()
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn progress(&self) -> f32 {
        let Some(duration) = self.duration_us else {
            return 0.0;
        };

        if duration <= 0 {
            return 0.0;
        }

        (self.position_us as f64 / duration as f64).clamp(0.0, 1.0) as f32
    }

    pub fn position_secs(&self) -> f32 {
        (self.position_us as f64 / 1_000_000.0) as f32
    }

    pub fn duration_secs(&self) -> f32 {
        let Some(duration) = self.duration_us else {
            return 0.0;
        };

        (duration as f64 / 1_000_000.0) as f32
    }

    pub fn toggle_playback(&mut self) {
        if self.playing {
            self.playing = false;
        } else {
            if self.finished {
                if let Err(error) = self.seek_to_relative_us(0) {
                    eprintln!(
                        "video restart after finish failed for {}: {error}",
                        self.path.display()
                    );
                }
            }

            self.playing = true;
        }

        self.last_tick = Instant::now();
        self.accumulator = Duration::ZERO;
    }

    pub fn restart_from_beginning(&mut self) -> Result<(), String> {
        self.seek_to_relative_us(0)
    }

    pub fn seek_to_progress(&mut self, progress: f32) -> Result<(), String> {
        let Some(duration) = self.duration_us else {
            return Ok(());
        };

        let clamped = progress.clamp(0.0, 1.0) as f64;
        let relative_target = (duration as f64 * clamped).round() as i64;
        self.seek_to_relative_us(relative_target)
    }

    pub fn tick(&mut self, now: Instant) {
        if !self.playing {
            self.last_tick = now;
            return;
        }

        let elapsed = now.saturating_duration_since(self.last_tick);
        self.last_tick = now;
        self.accumulator += elapsed;

        let mut steps = 0_usize;

        while self.playing && self.accumulator >= self.frame_interval && steps < 4 {
            if let Err(error) = self.decode_next_frame() {
                eprintln!("video decode error for {}: {error}", self.path.display());
                break;
            }

            self.accumulator = self.accumulator.saturating_sub(self.frame_interval);
            steps += 1;
        }
    }

    fn seek_to_relative_us(&mut self, relative_us: i64) -> Result<(), String> {
        let clamped_relative = if let Some(duration) = self.duration_us {
            relative_us.clamp(0, duration)
        } else {
            relative_us.max(0)
        };

        let target_us = self.stream_start_us.saturating_add(clamped_relative);

        self.input
            .seek(target_us, ..)
            .map_err(|error| format!("seek failed: {error}"))?;
        self.decoder.flush();
        self.accumulator = Duration::ZERO;
        self.last_tick = Instant::now();
        self.position_us = clamped_relative;
        self.pending_packet = None;
        self.eof_sent = false;
        self.finished = false;

        if let Err(error) = self.decode_next_frame() {
            eprintln!(
                "video decode error for {} after seek: {error}",
                self.path.display()
            );
        }

        Ok(())
    }

    fn decode_next_frame(&mut self) -> Result<(), String> {
        let mut decoded = ffmpeg::util::frame::video::Video::empty();

        loop {
            match self.decoder.receive_frame(&mut decoded) {
                Ok(()) => {
                    self.publish_frame(&decoded)?;
                    return Ok(());
                }
                Err(ffmpeg::Error::Other {
                    errno: ffmpeg::error::EAGAIN,
                }) => {}
                Err(ffmpeg::Error::Eof) => {
                    self.finish_on_last_frame();
                    return Ok(());
                }
                Err(error) => {
                    return Err(format!("receive_frame failed: {error}"));
                }
            }

            if let Some(packet) = self
                .pending_packet
                .take()
                .or_else(|| self.next_video_packet())
            {
                match self.decoder.send_packet(&packet) {
                    Ok(()) => {
                        self.eof_sent = false;
                    }
                    Err(ffmpeg::Error::Other {
                        errno: ffmpeg::error::EAGAIN,
                    }) => {
                        // Decoder input queue is full; retry this packet after draining.
                        self.pending_packet = Some(packet);
                    }
                    Err(error) => {
                        return Err(format!("send_packet failed: {error}"));
                    }
                }

                continue;
            }

            if !self.eof_sent {
                match self.decoder.send_eof() {
                    Ok(()) => {
                        self.eof_sent = true;
                    }
                    Err(ffmpeg::Error::Other {
                        errno: ffmpeg::error::EAGAIN,
                    }) => {
                        // Decoder still has output to drain.
                        self.eof_sent = true;
                    }
                    Err(error) => {
                        return Err(format!("send_eof failed: {error}"));
                    }
                }

                continue;
            }

            self.finish_on_last_frame();
            return Ok(());
        }
    }

    fn next_video_packet(&mut self) -> Option<ffmpeg::Packet> {
        loop {
            let item = self.input.packets().next()?;
            let (stream, packet) = item;

            if stream.index() == self.stream_index {
                return Some(packet);
            }
        }
    }

    fn finish_on_last_frame(&mut self) {
        if let Some(duration) = self.duration_us {
            self.position_us = duration;
        }

        self.playing = false;
        self.finished = true;
        self.accumulator = Duration::ZERO;
        self.pending_packet = None;
        self.eof_sent = false;
        self.last_tick = Instant::now();
    }

    fn publish_frame(&mut self, decoded: &ffmpeg::util::frame::video::Video) -> Result<(), String> {
        if let Some(timestamp) = decoded.timestamp().or(decoded.pts()) {
            if let Some(position_us) = self.position_us_from_stream_timestamp(timestamp) {
                self.position_us = position_us;
            }
        }

        let mut rgba = ffmpeg::util::frame::video::Video::empty();
        self.scaler
            .run(decoded, &mut rgba)
            .map_err(|error| format!("scale failed: {error}"))?;

        let width = rgba.width();
        let height = rgba.height();
        if width == 0 || height == 0 {
            return Ok(());
        }

        let stride = rgba.stride(0);
        let src = rgba.data(0);
        let row_len = width as usize * 4;

        let mut pixels = vec![0_u8; row_len * height as usize];

        for y in 0..height as usize {
            let src_offset = y * stride;
            let dst_offset = y * row_len;
            let src_end = src_offset + row_len;

            if src_end > src.len() {
                return Err("decoded frame row out of bounds".to_string());
            }

            pixels[dst_offset..dst_offset + row_len].copy_from_slice(&src[src_offset..src_end]);
        }

        // YUV sources do not carry alpha; force opaque output to avoid
        // random transparency flicker after colorspace conversion.
        if pixels.len() >= 4 {
            for alpha in pixels[3..].iter_mut().step_by(4) {
                *alpha = u8::MAX;
            }
        }

        let (output_width, output_height, rotated_pixels) =
            rotate_rgba(width, height, pixels, self.rotation);

        if output_width == 0 || output_height == 0 {
            return Ok(());
        }

        self.current_frame = Some(image::Handle::from_rgba(
            output_width,
            output_height,
            rotated_pixels,
        ));

        Ok(())
    }

    fn position_us_from_stream_timestamp(&self, timestamp: i64) -> Option<i64> {
        let absolute_us = timestamp_to_us(timestamp, self.stream_time_base)?;
        let relative_us = absolute_us.saturating_sub(self.stream_start_us).max(0);

        Some(match self.duration_us {
            Some(duration) => relative_us.min(duration),
            None => relative_us,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rotation {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

fn rotation_from_stream(stream: &ffmpeg::Stream<'_>) -> Rotation {
    rotation_from_display_matrix(stream)
        .or_else(|| rotation_from_rotate_metadata(stream))
        .unwrap_or(Rotation::Deg0)
}

fn rotation_from_display_matrix(stream: &ffmpeg::Stream<'_>) -> Option<Rotation> {
    let matrix_type = ffmpeg::codec::packet::side_data::Type::DisplayMatrix;

    for side_data in stream.side_data() {
        if side_data.kind() != matrix_type {
            continue;
        }

        let degrees = rotation_degrees_from_display_matrix(side_data.data())?;
        return Some(rotation_from_degrees(degrees));
    }

    None
}

fn rotation_from_rotate_metadata(stream: &ffmpeg::Stream<'_>) -> Option<Rotation> {
    let metadata = stream.metadata();
    let raw = metadata.get("rotate")?;
    let parsed = raw.parse::<f32>().ok()?;

    Some(rotation_from_degrees(parsed))
}

fn rotation_from_degrees(parsed: f32) -> Rotation {
    let (degrees, _) = nearest_right_angle(parsed as f64);

    // Existing renderer rotation orientation mapping.
    match degrees {
        90 => Rotation::Deg270,
        180 => Rotation::Deg180,
        270 => Rotation::Deg90,
        _ => Rotation::Deg0,
    }
}

fn rotation_degrees_from_display_matrix(data: &[u8]) -> Option<f32> {
    let little = read_display_matrix(data, i32::from_le_bytes)?;
    let big = read_display_matrix(data, i32::from_be_bytes)?;

    let little_rotation = display_matrix_rotation_degrees(&little)?;
    let big_rotation = display_matrix_rotation_degrees(&big)?;

    let (_, little_error) = nearest_right_angle(little_rotation);
    let (_, big_error) = nearest_right_angle(big_rotation);

    Some(if little_error <= big_error {
        little_rotation as f32
    } else {
        big_rotation as f32
    })
}

fn read_display_matrix(data: &[u8], parse_word: fn([u8; 4]) -> i32) -> Option<[i32; 9]> {
    if data.len() < 9 * 4 {
        return None;
    }

    let mut matrix = [0_i32; 9];

    for (index, chunk) in data[..9 * 4].chunks_exact(4).enumerate() {
        matrix[index] = parse_word(chunk.try_into().ok()?);
    }

    Some(matrix)
}

fn display_matrix_rotation_degrees(matrix: &[i32; 9]) -> Option<f64> {
    let conv = |value: i32| value as f64 / 65536.0;

    let scale0 = (conv(matrix[0]).powi(2) + conv(matrix[3]).powi(2)).sqrt();
    let scale1 = (conv(matrix[1]).powi(2) + conv(matrix[4]).powi(2)).sqrt();

    if scale0 <= f64::EPSILON || scale1 <= f64::EPSILON {
        return None;
    }

    let rotation = -(conv(matrix[1]) / scale1)
        .atan2(conv(matrix[0]) / scale0)
        .to_degrees();

    Some(rotation.rem_euclid(360.0))
}

fn nearest_right_angle(degrees: f64) -> (i32, f64) {
    let normalized = degrees.rem_euclid(360.0);
    let targets = [0_i32, 90, 180, 270];

    let mut best = (0_i32, f64::INFINITY);

    for target in targets {
        let target_f = target as f64;
        let distance = (normalized - target_f).abs();
        let wrapped_distance = distance.min(360.0 - distance);

        if wrapped_distance < best.1 {
            best = (target, wrapped_distance);
        }
    }

    best
}

fn rotate_rgba(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    rotation: Rotation,
) -> (u32, u32, Vec<u8>) {
    match rotation {
        Rotation::Deg0 => (width, height, pixels),
        Rotation::Deg90 => {
            let mut out = vec![0_u8; pixels.len()];
            let src_w = width as usize;
            let src_h = height as usize;
            let dst_w = src_h;

            for y in 0..src_h {
                for x in 0..src_w {
                    let src_idx = (y * src_w + x) * 4;
                    let dx = src_h - 1 - y;
                    let dy = x;
                    let dst_idx = (dy * dst_w + dx) * 4;
                    out[dst_idx..dst_idx + 4].copy_from_slice(&pixels[src_idx..src_idx + 4]);
                }
            }

            (height, width, out)
        }
        Rotation::Deg180 => {
            let mut out = vec![0_u8; pixels.len()];
            let src_w = width as usize;
            let src_h = height as usize;

            for y in 0..src_h {
                for x in 0..src_w {
                    let src_idx = (y * src_w + x) * 4;
                    let dx = src_w - 1 - x;
                    let dy = src_h - 1 - y;
                    let dst_idx = (dy * src_w + dx) * 4;
                    out[dst_idx..dst_idx + 4].copy_from_slice(&pixels[src_idx..src_idx + 4]);
                }
            }

            (width, height, out)
        }
        Rotation::Deg270 => {
            let mut out = vec![0_u8; pixels.len()];
            let src_w = width as usize;
            let src_h = height as usize;
            let dst_w = src_h;

            for y in 0..src_h {
                for x in 0..src_w {
                    let src_idx = (y * src_w + x) * 4;
                    let dx = y;
                    let dy = src_w - 1 - x;
                    let dst_idx = (dy * dst_w + dx) * 4;
                    out[dst_idx..dst_idx + 4].copy_from_slice(&pixels[src_idx..src_idx + 4]);
                }
            }

            (height, width, out)
        }
    }
}

fn scaled_output_size(width: u32, height: u32, rotation: Rotation) -> (u32, u32) {
    let (display_width, display_height) = match rotation {
        Rotation::Deg90 | Rotation::Deg270 => (height as f32, width as f32),
        Rotation::Deg0 | Rotation::Deg180 => (width as f32, height as f32),
    };

    let scale = (MAX_FRAME_WIDTH as f32 / display_width)
        .min(MAX_FRAME_HEIGHT as f32 / display_height)
        .min(1.0);

    let out_display_width = (display_width * scale).round().max(1.0) as u32;
    let out_display_height = (display_height * scale).round().max(1.0) as u32;

    match rotation {
        Rotation::Deg90 | Rotation::Deg270 => (out_display_height, out_display_width),
        Rotation::Deg0 | Rotation::Deg180 => (out_display_width, out_display_height),
    }
}

fn ensure_ffmpeg_initialized() -> Result<(), String> {
    static INIT: OnceLock<Result<(), String>> = OnceLock::new();

    match INIT.get_or_init(|| {
        ffmpeg::init().map_err(|error| format!("ffmpeg init failed: {error}"))?;
        ffmpeg::log::set_level(ffmpeg::log::Level::Error);
        Ok(())
    }) {
        Ok(()) => Ok(()),
        Err(error) => Err(error.clone()),
    }
}

fn read_container_duration_us(input: &ffmpeg::format::context::Input) -> Option<i64> {
    valid_duration_us(input.duration())
}

fn read_stream_duration_us(stream: &ffmpeg::Stream<'_>) -> Option<i64> {
    let duration = non_nopts(stream.duration())?;
    let duration_us = timestamp_to_us(duration, stream.time_base())?;
    valid_duration_us(duration_us)
}

fn read_stream_start_us(stream: &ffmpeg::Stream<'_>) -> Option<i64> {
    let start = non_nopts(stream.start_time())?;
    timestamp_to_us(start, stream.time_base())
}

fn timestamp_to_us(value: i64, time_base: ffmpeg::Rational) -> Option<i64> {
    let numerator = time_base.numerator() as f64;
    let denominator = time_base.denominator() as f64;

    if denominator <= 0.0 {
        return None;
    }

    let seconds = value as f64 * numerator / denominator;
    Some((seconds * 1_000_000.0).round() as i64)
}

fn non_nopts(value: i64) -> Option<i64> {
    if value == ffmpeg::ffi::AV_NOPTS_VALUE {
        None
    } else {
        Some(value)
    }
}

fn valid_duration_us(value: i64) -> Option<i64> {
    if value > 0 { Some(value) } else { None }
}

fn rational_to_fps(rate: ffmpeg::Rational) -> Option<f32> {
    let numerator = rate.numerator();
    let denominator = rate.denominator();

    if numerator > 0 && denominator > 0 {
        Some(numerator as f32 / denominator as f32)
    } else {
        None
    }
}
