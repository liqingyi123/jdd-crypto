//! Physical display brightness: Windows WMI (laptop) + DDC/CI (external).

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayBrightness {
    pub id: String,
    pub name: String,
    pub brightness: u32,
    pub min: u32,
    pub max: u32,
    pub supported: bool,
}

pub fn list_displays() -> Result<Vec<DisplayBrightness>, String> {
    #[cfg(windows)]
    {
        return win::list_displays();
    }
    #[cfg(target_os = "macos")]
    {
        return macos::list_displays();
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Ok(Vec::new())
    }
}

pub fn set_display_brightness(id: &str, value: u32) -> Result<(), String> {
    #[cfg(windows)]
    {
        return win::set_display_brightness(id, value);
    }
    #[cfg(target_os = "macos")]
    {
        return macos::set_display_brightness(id, value);
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (id, value);
        Err("当前平台不支持调节亮度".to_string())
    }
}

#[cfg(windows)]
mod win {
    use super::DisplayBrightness;
    use serde::Deserialize;
    use std::sync::Mutex;
    use windows_sys::Win32::Devices::Display::{
        DestroyPhysicalMonitors, GetMonitorBrightness, GetNumberOfPhysicalMonitorsFromHMONITOR,
        GetPhysicalMonitorsFromHMONITOR, PHYSICAL_MONITOR, SetMonitorBrightness,
    };
    use windows_sys::Win32::Foundation::{BOOL, LPARAM, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
    };

    #[derive(Clone)]
    enum Backend {
        Ddc { hmonitor: isize },
        Wmi { instance: String },
    }

    #[derive(Clone)]
    struct CachedDisplay {
        id: String,
        backend: Backend,
    }

    static CACHE: Mutex<Vec<CachedDisplay>> = Mutex::new(Vec::new());

    #[derive(Deserialize, Debug)]
    #[serde(rename = "WmiMonitorBrightness")]
    #[serde(rename_all = "PascalCase")]
    struct WmiBrightness {
        current_brightness: u8,
        instance_name: String,
    }

    struct EnumState {
        items: Vec<(HMONITOR, String)>,
    }

    unsafe extern "system" fn monitor_enum_proc(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let state = &mut *(lparam as *mut EnumState);
        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(hmonitor, &mut info as *mut _ as *mut _) != 0 {
            let len = info
                .szDevice
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(info.szDevice.len());
            let name = String::from_utf16_lossy(&info.szDevice[..len]);
            state.items.push((hmonitor, name));
        }
        1
    }

    fn enumerate_monitors() -> Vec<(HMONITOR, String)> {
        let mut state = EnumState { items: Vec::new() };
        unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(monitor_enum_proc),
                &mut state as *mut _ as LPARAM,
            );
        }
        state.items
    }

    fn read_ddc(hmonitor: HMONITOR) -> Option<(u32, u32, u32, String)> {
        unsafe {
            let mut count: u32 = 0;
            if GetNumberOfPhysicalMonitorsFromHMONITOR(hmonitor, &mut count) == 0 || count == 0 {
                return None;
            }
            let mut monitors = vec![std::mem::zeroed::<PHYSICAL_MONITOR>(); count as usize];
            if GetPhysicalMonitorsFromHMONITOR(hmonitor, count, monitors.as_mut_ptr()) == 0 {
                return None;
            }
            let first = monitors[0];
            let mut min = 0u32;
            let mut cur = 0u32;
            let mut max = 0u32;
            let ok = GetMonitorBrightness(first.hPhysicalMonitor, &mut min, &mut cur, &mut max);
            let desc_chars = first.szPhysicalMonitorDescription;
            let desc_len = desc_chars
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(desc_chars.len());
            let desc = String::from_utf16_lossy(&desc_chars[..desc_len]);
            DestroyPhysicalMonitors(count, monitors.as_mut_ptr());
            if ok == 0 {
                return None;
            }
            Some((min, cur, max.max(1), desc))
        }
    }

    fn set_ddc(hmonitor: HMONITOR, value: u32) -> Result<(), String> {
        unsafe {
            let mut count: u32 = 0;
            if GetNumberOfPhysicalMonitorsFromHMONITOR(hmonitor, &mut count) == 0 || count == 0 {
                return Err("无法访问显示器".to_string());
            }
            let mut monitors = vec![std::mem::zeroed::<PHYSICAL_MONITOR>(); count as usize];
            if GetPhysicalMonitorsFromHMONITOR(hmonitor, count, monitors.as_mut_ptr()) == 0 {
                return Err("无法获取物理显示器".to_string());
            }
            let handle = monitors[0].hPhysicalMonitor;
            let mut min = 0u32;
            let mut cur = 0u32;
            let mut max = 100u32;
            let _ = GetMonitorBrightness(handle, &mut min, &mut cur, &mut max);
            let clamped = value.clamp(min, max.max(min));
            let ok = SetMonitorBrightness(handle, clamped);
            DestroyPhysicalMonitors(count, monitors.as_mut_ptr());
            if ok == 0 {
                return Err("设置亮度失败（显示器可能不支持 DDC/CI）".to_string());
            }
            Ok(())
        }
    }

    fn list_wmi() -> Vec<(String, u8)> {
        let Ok(com) = wmi::COMLibrary::new() else {
            return Vec::new();
        };
        let Ok(wmi_con) = wmi::WMIConnection::with_namespace_path("root\\WMI", com) else {
            return Vec::new();
        };
        let Ok(rows): Result<Vec<WmiBrightness>, _> = wmi_con.query() else {
            return Vec::new();
        };
        rows.into_iter()
            .map(|row| (row.instance_name, row.current_brightness))
            .collect()
    }

    fn set_wmi(instance: &str, value: u32) -> Result<(), String> {
        let brightness = value.min(100);
        let escaped = instance.replace('\'', "''");
        let script = format!(
            "$m = Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods | Where-Object {{ $_.InstanceName -eq '{escaped}' }}; if ($null -eq $m) {{ throw 'not found' }}; Invoke-CimMethod -InputObject $m -MethodName WmiSetBrightness -Arguments @{{Timeout=1; Brightness={brightness}}} | Out-Null"
        );
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
            .output()
            .map_err(|e| format!("调节内建亮度失败: {e}"))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            let brief: String = err.trim().chars().take(160).collect();
            return Err(if brief.is_empty() {
                "调节内建亮度失败".to_string()
            } else {
                format!("调节内建亮度失败: {brief}")
            });
        }
        Ok(())
    }

    pub fn list_displays() -> Result<Vec<DisplayBrightness>, String> {
        let mut out = Vec::new();
        let mut cache = Vec::new();
        let mut idx = 0u32;

        for (hmonitor, device_name) in enumerate_monitors() {
            let id = format!("disp-{idx}");
            if let Some((min, cur, max, desc)) = read_ddc(hmonitor) {
                let name = if desc.trim().is_empty() {
                    format!("显示器 {}", idx + 1)
                } else {
                    desc
                };
                out.push(DisplayBrightness {
                    id: id.clone(),
                    name,
                    brightness: cur,
                    min,
                    max,
                    supported: true,
                });
                cache.push(CachedDisplay {
                    id,
                    backend: Backend::Ddc {
                        hmonitor: hmonitor as isize,
                    },
                });
            } else {
                let name = if device_name.trim().is_empty() {
                    format!("显示器 {}", idx + 1)
                } else {
                    device_name
                };
                out.push(DisplayBrightness {
                    id: id.clone(),
                    name,
                    brightness: 50,
                    min: 0,
                    max: 100,
                    supported: false,
                });
                cache.push(CachedDisplay {
                    id,
                    backend: Backend::Ddc {
                        hmonitor: hmonitor as isize,
                    },
                });
            }
            idx += 1;
        }

        let wmi_rows = list_wmi();
        for (instance, cur) in wmi_rows {
            // Bind WMI to the first unsupported entry (typical laptop panel).
            if let Some(pos) = out.iter().position(|d| !d.supported) {
                let id = out[pos].id.clone();
                out[pos].brightness = u32::from(cur);
                out[pos].min = 0;
                out[pos].max = 100;
                out[pos].supported = true;
                if out[pos].name.starts_with(r"\\.\DISPLAY") {
                    out[pos].name = format!("内建屏幕 {}", pos + 1);
                }
                if let Some(c) = cache.iter_mut().find(|c| c.id == id) {
                    c.backend = Backend::Wmi { instance };
                }
            } else {
                let id = format!("disp-{idx}");
                out.push(DisplayBrightness {
                    id: id.clone(),
                    name: format!("内建屏幕 {}", out.len() + 1),
                    brightness: u32::from(cur),
                    min: 0,
                    max: 100,
                    supported: true,
                });
                cache.push(CachedDisplay {
                    id,
                    backend: Backend::Wmi { instance },
                });
                idx += 1;
            }
        }

        if let Ok(mut guard) = CACHE.lock() {
            *guard = cache;
        }
        Ok(out)
    }

    pub fn set_display_brightness(id: &str, value: u32) -> Result<(), String> {
        let cache = CACHE
            .lock()
            .map_err(|_| "亮度缓存锁定失败".to_string())?
            .clone();
        let entry = cache
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| "未找到显示器，请重新打开亮度面板".to_string())?;
        match &entry.backend {
            Backend::Ddc { hmonitor } => {
                // Prefer DDC; if it fails and we only had a placeholder, surface error.
                set_ddc(*hmonitor as HMONITOR, value)
            }
            Backend::Wmi { instance } => set_wmi(instance, value),
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::DisplayBrightness;
    use core_graphics::display::CGDisplay;

    pub fn list_displays() -> Result<Vec<DisplayBrightness>, String> {
        let main = CGDisplay::main();
        let id = format!("cg-{}", main.id);
        Ok(vec![DisplayBrightness {
            id,
            name: "主显示器".to_string(),
            brightness: 50,
            min: 0,
            max: 100,
            supported: false,
        }])
    }

    pub fn set_display_brightness(_id: &str, _value: u32) -> Result<(), String> {
        Err("当前 macOS 版本暂不支持调节物理亮度".to_string())
    }
}
