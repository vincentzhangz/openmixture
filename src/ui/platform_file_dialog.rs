use std::path::PathBuf;

use super::model::InputTemplate;

pub fn pick_media_with_system_dialog(template: InputTemplate) -> Option<PathBuf> {
    pick_media_with_system_dialog_impl(template)
}

#[cfg(target_os = "macos")]
fn pick_media_with_system_dialog_impl(template: InputTemplate) -> Option<PathBuf> {
    use std::process::Command;

    let media_filter = match template {
        InputTemplate::Video => r#"{"public.movie"}"#,
        InputTemplate::Photo => r#"{"public.image"}"#,
        InputTemplate::Black | InputTemplate::ScreenBar => return None,
    };

    let script = format!(
        "try\nPOSIX path of (choose file of type {media_filter})\non error number -128\n\"\"\nend try"
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let picked = String::from_utf8(output.stdout).ok()?;
    let picked = picked.trim();

    if picked.is_empty() {
        None
    } else {
        Some(PathBuf::from(picked))
    }
}

#[cfg(target_os = "windows")]
fn pick_media_with_system_dialog_impl(template: InputTemplate) -> Option<PathBuf> {
    use std::process::Command;

    let filter = match template {
        InputTemplate::Video => "Video Files|*.mp4;*.mov;*.mkv;*.webm;*.avi;*.m4v|All Files|*.*",
        InputTemplate::Photo => "Image Files|*.jpg;*.jpeg;*.png;*.bmp;*.webp;*.gif|All Files|*.*",
        InputTemplate::Black | InputTemplate::ScreenBar => return None,
    };

    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms\n\
         $dialog = New-Object System.Windows.Forms.OpenFileDialog\n\
         $dialog.Filter = \"{filter}\"\n\
         $dialog.Multiselect = $false\n\
         if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{\n\
             Write-Output $dialog.FileName\n\
         }}"
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .ok()
        .or_else(|| {
            Command::new("pwsh")
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .output()
                .ok()
        })?;

    if !output.status.success() {
        return None;
    }

    let picked = String::from_utf8(output.stdout).ok()?;
    let picked = picked.trim();

    if picked.is_empty() {
        None
    } else {
        Some(PathBuf::from(picked))
    }
}

#[cfg(target_os = "linux")]
fn pick_media_with_system_dialog_impl(template: InputTemplate) -> Option<PathBuf> {
    use std::process::Command;

    let (title, zenity_filter, kdialog_filter) = match template {
        InputTemplate::Video => (
            "Select Video File",
            "Video files | *.mp4 *.mov *.mkv *.webm *.avi *.m4v",
            "*.mp4 *.mov *.mkv *.webm *.avi *.m4v|Video files",
        ),
        InputTemplate::Photo => (
            "Select Image File",
            "Image files | *.jpg *.jpeg *.png *.bmp *.webp *.gif",
            "*.jpg *.jpeg *.png *.bmp *.webp *.gif|Image files",
        ),
        InputTemplate::Black | InputTemplate::ScreenBar => return None,
    };

    let script = format!(
        "if command -v zenity >/dev/null 2>&1; then\n\
             zenity --file-selection --title='{title}' --file-filter='{zenity_filter}' 2>/dev/null\n\
         elif command -v kdialog >/dev/null 2>&1; then\n\
             kdialog --getopenfilename \"$HOME\" '{kdialog_filter}' --title '{title}' 2>/dev/null\n\
         fi"
    );

    let output = Command::new("sh").args(["-c", &script]).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let picked = String::from_utf8(output.stdout).ok()?;
    let picked = picked.trim();

    if picked.is_empty() {
        None
    } else {
        Some(PathBuf::from(picked))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn pick_media_with_system_dialog_impl(_template: InputTemplate) -> Option<PathBuf> {
    None
}
