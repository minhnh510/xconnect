use crate::HostRuntimeError;

#[derive(Debug, Clone)]
pub struct EncoderConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveEncoderBackend {
    Software,
    #[cfg(target_os = "windows")]
    MediaFoundation,
    #[cfg(target_os = "windows")]
    Nvenc,
    #[cfg(target_os = "windows")]
    Qsv,
    #[cfg(target_os = "macos")]
    VideoToolbox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendPreference {
    Auto,
    Software,
    MediaFoundation,
    Nvenc,
    Qsv,
    VideoToolbox,
}

impl BackendPreference {
    fn from_env() -> Self {
        let raw = std::env::var("XCONNECT_H264_BACKEND")
            .unwrap_or_else(|_| "auto".to_string())
            .to_lowercase();

        match raw.as_str() {
            "software" => Self::Software,
            "media_foundation" | "mf" => Self::MediaFoundation,
            "nvenc" => Self::Nvenc,
            "qsv" => Self::Qsv,
            "videotoolbox" | "vt" => Self::VideoToolbox,
            _ => Self::Auto,
        }
    }
}

pub struct H264Encoder {
    backend: EncoderBackend,
    active_backend: ActiveEncoderBackend,
}

enum EncoderBackend {
    Software(SoftwareEncoder),
    #[cfg(target_os = "windows")]
    MediaFoundation(WindowsMediaFoundationEncoder),
    #[cfg(target_os = "windows")]
    Nvenc(WindowsNvencEncoder),
    #[cfg(target_os = "windows")]
    Qsv(WindowsQsvEncoder),
    #[cfg(target_os = "macos")]
    VideoToolbox(MacosVideoToolboxEncoder),
}

impl H264Encoder {
    pub fn new(config: EncoderConfig) -> Result<Self, HostRuntimeError> {
        validate_config(&config)?;
        let preference = BackendPreference::from_env();
        let (backend, active_backend) = build_backend(config, preference)?;
        Ok(Self {
            backend,
            active_backend,
        })
    }

    pub fn active_backend(&self) -> ActiveEncoderBackend {
        self.active_backend
    }

    pub fn encode(&mut self, frame_rgba: &[u8]) -> Result<Vec<u8>, HostRuntimeError> {
        match &mut self.backend {
            EncoderBackend::Software(enc) => enc.encode(frame_rgba),
            #[cfg(target_os = "windows")]
            EncoderBackend::MediaFoundation(enc) => enc.encode(frame_rgba),
            #[cfg(target_os = "windows")]
            EncoderBackend::Nvenc(enc) => enc.encode(frame_rgba),
            #[cfg(target_os = "windows")]
            EncoderBackend::Qsv(enc) => enc.encode(frame_rgba),
            #[cfg(target_os = "macos")]
            EncoderBackend::VideoToolbox(enc) => enc.encode(frame_rgba),
        }
    }
}

fn validate_config(config: &EncoderConfig) -> Result<(), HostRuntimeError> {
    if config.width == 0 || config.height == 0 || config.fps == 0 {
        return Err(HostRuntimeError::Runtime(
            "invalid encoder config".to_string(),
        ));
    }
    Ok(())
}

fn build_backend(
    config: EncoderConfig,
    preference: BackendPreference,
) -> Result<(EncoderBackend, ActiveEncoderBackend), HostRuntimeError> {
    #[cfg(target_os = "windows")]
    {
        return build_windows_backend(config, preference);
    }

    #[cfg(target_os = "macos")]
    {
        return build_macos_backend(config, preference);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let software = SoftwareEncoder::new(config)?;
        Ok((
            EncoderBackend::Software(software),
            ActiveEncoderBackend::Software,
        ))
    }
}

#[cfg(target_os = "windows")]
fn build_windows_backend(
    config: EncoderConfig,
    preference: BackendPreference,
) -> Result<(EncoderBackend, ActiveEncoderBackend), HostRuntimeError> {
    match preference {
        BackendPreference::Software => {
            let software = SoftwareEncoder::new(config)?;
            Ok((
                EncoderBackend::Software(software),
                ActiveEncoderBackend::Software,
            ))
        }
        BackendPreference::Nvenc => {
            if let Ok(encoder) = WindowsNvencEncoder::new(config.clone()) {
                return Ok((EncoderBackend::Nvenc(encoder), ActiveEncoderBackend::Nvenc));
            }
            if let Ok(encoder) = WindowsMediaFoundationEncoder::new(config.clone()) {
                return Ok((
                    EncoderBackend::MediaFoundation(encoder),
                    ActiveEncoderBackend::MediaFoundation,
                ));
            }
            let software = SoftwareEncoder::new(config)?;
            Ok((
                EncoderBackend::Software(software),
                ActiveEncoderBackend::Software,
            ))
        }
        BackendPreference::Qsv => {
            if let Ok(encoder) = WindowsQsvEncoder::new(config.clone()) {
                return Ok((EncoderBackend::Qsv(encoder), ActiveEncoderBackend::Qsv));
            }
            if let Ok(encoder) = WindowsMediaFoundationEncoder::new(config.clone()) {
                return Ok((
                    EncoderBackend::MediaFoundation(encoder),
                    ActiveEncoderBackend::MediaFoundation,
                ));
            }
            let software = SoftwareEncoder::new(config)?;
            Ok((
                EncoderBackend::Software(software),
                ActiveEncoderBackend::Software,
            ))
        }
        BackendPreference::MediaFoundation => {
            if let Ok(encoder) = WindowsMediaFoundationEncoder::new(config.clone()) {
                return Ok((
                    EncoderBackend::MediaFoundation(encoder),
                    ActiveEncoderBackend::MediaFoundation,
                ));
            }
            let software = SoftwareEncoder::new(config)?;
            Ok((
                EncoderBackend::Software(software),
                ActiveEncoderBackend::Software,
            ))
        }
        BackendPreference::Auto => {
            if let Ok(encoder) = WindowsNvencEncoder::new(config.clone()) {
                return Ok((EncoderBackend::Nvenc(encoder), ActiveEncoderBackend::Nvenc));
            }
            if let Ok(encoder) = WindowsQsvEncoder::new(config.clone()) {
                return Ok((EncoderBackend::Qsv(encoder), ActiveEncoderBackend::Qsv));
            }
            if let Ok(encoder) = WindowsMediaFoundationEncoder::new(config.clone()) {
                return Ok((
                    EncoderBackend::MediaFoundation(encoder),
                    ActiveEncoderBackend::MediaFoundation,
                ));
            }
            let software = SoftwareEncoder::new(config)?;
            Ok((
                EncoderBackend::Software(software),
                ActiveEncoderBackend::Software,
            ))
        }
        BackendPreference::VideoToolbox => {
            let software = SoftwareEncoder::new(config)?;
            Ok((
                EncoderBackend::Software(software),
                ActiveEncoderBackend::Software,
            ))
        }
    }
}

#[cfg(target_os = "macos")]
fn build_macos_backend(
    config: EncoderConfig,
    preference: BackendPreference,
) -> Result<(EncoderBackend, ActiveEncoderBackend), HostRuntimeError> {
    match preference {
        BackendPreference::Software => {
            let software = SoftwareEncoder::new(config)?;
            Ok((
                EncoderBackend::Software(software),
                ActiveEncoderBackend::Software,
            ))
        }
        BackendPreference::VideoToolbox | BackendPreference::Auto => {
            if let Ok(encoder) = MacosVideoToolboxEncoder::new(config.clone()) {
                return Ok((
                    EncoderBackend::VideoToolbox(encoder),
                    ActiveEncoderBackend::VideoToolbox,
                ));
            }
            let software = SoftwareEncoder::new(config)?;
            Ok((
                EncoderBackend::Software(software),
                ActiveEncoderBackend::Software,
            ))
        }
        BackendPreference::MediaFoundation | BackendPreference::Nvenc | BackendPreference::Qsv => {
            let software = SoftwareEncoder::new(config)?;
            Ok((
                EncoderBackend::Software(software),
                ActiveEncoderBackend::Software,
            ))
        }
    }
}

#[derive(Clone)]
struct SoftwareEncoder {
    config: EncoderConfig,
}

impl SoftwareEncoder {
    fn new(config: EncoderConfig) -> Result<Self, HostRuntimeError> {
        validate_config(&config)?;
        Ok(Self { config })
    }

    fn encode(&mut self, frame_rgba: &[u8]) -> Result<Vec<u8>, HostRuntimeError> {
        let expected = (self.config.width * self.config.height * 4) as usize;
        if frame_rgba.len() != expected {
            return Err(HostRuntimeError::Runtime(format!(
                "invalid rgba length: expected {expected}, got {}",
                frame_rgba.len()
            )));
        }

        // Software fallback payload to keep the rest of the pipeline alive.
        // Hardware backends produce Annex-B H.264 instead.
        let mut rgb = Vec::with_capacity((self.config.width * self.config.height * 3) as usize);
        for chunk in frame_rgba.chunks_exact(4) {
            rgb.extend_from_slice(&chunk[..3]);
        }
        Ok(rgb)
    }
}

fn has_annex_b_start_code(payload: &[u8]) -> bool {
    payload.windows(4).any(|w| w == [0, 0, 0, 1]) || payload.windows(3).any(|w| w == [0, 0, 1])
}

fn avcc_to_annex_b(payload: &[u8]) -> Result<Vec<u8>, HostRuntimeError> {
    for len_size in [4usize, 2usize, 1usize] {
        if let Some(converted) = try_avcc_to_annex_b(payload, len_size) {
            if !converted.is_empty() {
                return Ok(converted);
            }
        }
    }

    Err(HostRuntimeError::Runtime(
        "unable to parse H.264 payload as Annex-B or AVCC".to_string(),
    ))
}

fn try_avcc_to_annex_b(payload: &[u8], len_size: usize) -> Option<Vec<u8>> {
    let mut offset = 0usize;
    let mut out = Vec::with_capacity(payload.len() + 64);

    while offset + len_size <= payload.len() {
        let nal_len = match len_size {
            4 => u32::from_be_bytes([
                payload[offset],
                payload[offset + 1],
                payload[offset + 2],
                payload[offset + 3],
            ]) as usize,
            2 => u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize,
            1 => payload[offset] as usize,
            _ => return None,
        };
        offset += len_size;

        if nal_len == 0 {
            continue;
        }

        if offset + nal_len > payload.len() {
            return None;
        }

        out.extend_from_slice(&[0, 0, 0, 1]);
        out.extend_from_slice(&payload[offset..offset + nal_len]);
        offset += nal_len;
    }

    if offset == payload.len() {
        Some(out)
    } else {
        None
    }
}

fn ensure_annex_b(payload: &[u8]) -> Result<Vec<u8>, HostRuntimeError> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }
    if has_annex_b_start_code(payload) {
        return Ok(payload.to_vec());
    }
    avcc_to_annex_b(payload)
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MfVendorFilter {
    Any,
    Nvidia,
    Intel,
}

#[cfg(target_os = "windows")]
struct WindowsMediaFoundationEncoder {
    inner: HardwareMftEncoder,
}

#[cfg(target_os = "windows")]
impl WindowsMediaFoundationEncoder {
    fn new(config: EncoderConfig) -> Result<Self, HostRuntimeError> {
        Ok(Self {
            inner: HardwareMftEncoder::new(config, MfVendorFilter::Any)?,
        })
    }

    fn encode(&mut self, frame_rgba: &[u8]) -> Result<Vec<u8>, HostRuntimeError> {
        self.inner.encode(frame_rgba)
    }
}

#[cfg(target_os = "windows")]
struct WindowsNvencEncoder {
    inner: HardwareMftEncoder,
}

#[cfg(target_os = "windows")]
impl WindowsNvencEncoder {
    fn new(config: EncoderConfig) -> Result<Self, HostRuntimeError> {
        Ok(Self {
            inner: HardwareMftEncoder::new(config, MfVendorFilter::Nvidia)?,
        })
    }

    fn encode(&mut self, frame_rgba: &[u8]) -> Result<Vec<u8>, HostRuntimeError> {
        self.inner.encode(frame_rgba)
    }
}

#[cfg(target_os = "windows")]
struct WindowsQsvEncoder {
    inner: HardwareMftEncoder,
}

#[cfg(target_os = "windows")]
impl WindowsQsvEncoder {
    fn new(config: EncoderConfig) -> Result<Self, HostRuntimeError> {
        Ok(Self {
            inner: HardwareMftEncoder::new(config, MfVendorFilter::Intel)?,
        })
    }

    fn encode(&mut self, frame_rgba: &[u8]) -> Result<Vec<u8>, HostRuntimeError> {
        self.inner.encode(frame_rgba)
    }
}

#[cfg(target_os = "windows")]
struct MfRuntime {
    com_initialized: bool,
    mf_initialized: bool,
}

#[cfg(target_os = "windows")]
impl MfRuntime {
    fn startup() -> Result<Self, HostRuntimeError> {
        use windows::Win32::{
            Foundation::RPC_E_CHANGED_MODE,
            Media::MediaFoundation::{MFStartup, MFSTARTUP_NOSOCKET, MF_VERSION},
            System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
        };

        let com_result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let com_initialized = if com_result.is_ok() {
            true
        } else if com_result == RPC_E_CHANGED_MODE {
            false
        } else {
            return Err(HostRuntimeError::Runtime(format!(
                "CoInitializeEx failed: {com_result:?}"
            )));
        };

        unsafe { MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET) }
            .map_err(|err| HostRuntimeError::Runtime(format!("MFStartup failed: {err}")))?;

        Ok(Self {
            com_initialized,
            mf_initialized: true,
        })
    }
}

#[cfg(target_os = "windows")]
impl Drop for MfRuntime {
    fn drop(&mut self) {
        use windows::Win32::{Media::MediaFoundation::MFShutdown, System::Com::CoUninitialize};

        if self.mf_initialized {
            let _ = unsafe { MFShutdown() };
            self.mf_initialized = false;
        }

        if self.com_initialized {
            unsafe { CoUninitialize() };
            self.com_initialized = false;
        }
    }
}

#[cfg(target_os = "windows")]
struct HardwareMftEncoder {
    _mf_runtime: MfRuntime,
    transform: windows::Win32::Media::MediaFoundation::IMFTransform,
    config: EncoderConfig,
    input_stream_id: u32,
    output_stream_id: u32,
    output_stream_info: windows::Win32::Media::MediaFoundation::MFT_OUTPUT_STREAM_INFO,
    frame_index: u64,
    frame_duration_hns: i64,
}

#[cfg(target_os = "windows")]
impl HardwareMftEncoder {
    fn new(config: EncoderConfig, vendor: MfVendorFilter) -> Result<Self, HostRuntimeError> {
        validate_config(&config)?;
        if config.width % 2 != 0 || config.height % 2 != 0 {
            return Err(HostRuntimeError::Runtime(
                "hardware H.264 encoder requires even width and height".to_string(),
            ));
        }

        let mf_runtime = MfRuntime::startup()?;
        let transform = find_hardware_h264_encoder(vendor)?;
        configure_encoder_types(&transform, &config)?;
        let (input_stream_id, output_stream_id) = detect_stream_ids(&transform)?;
        let output_stream_info = unsafe { transform.GetOutputStreamInfo(output_stream_id) }
            .map_err(|err| {
                HostRuntimeError::Runtime(format!("GetOutputStreamInfo failed: {err}"))
            })?;

        unsafe {
            let _ = transform.ProcessMessage(
                windows::Win32::Media::MediaFoundation::MFT_MESSAGE_COMMAND_FLUSH,
                0,
            );
            transform
                .ProcessMessage(
                    windows::Win32::Media::MediaFoundation::MFT_MESSAGE_NOTIFY_BEGIN_STREAMING,
                    0,
                )
                .map_err(|err| {
                    HostRuntimeError::Runtime(format!(
                        "ProcessMessage(BEGIN_STREAMING) failed: {err}"
                    ))
                })?;
            transform
                .ProcessMessage(
                    windows::Win32::Media::MediaFoundation::MFT_MESSAGE_NOTIFY_START_OF_STREAM,
                    0,
                )
                .map_err(|err| {
                    HostRuntimeError::Runtime(format!(
                        "ProcessMessage(START_OF_STREAM) failed: {err}"
                    ))
                })?;
        }

        let frame_duration_hns = 10_000_000i64 / i64::from(config.fps.max(1));

        Ok(Self {
            _mf_runtime: mf_runtime,
            transform,
            config,
            input_stream_id,
            output_stream_id,
            output_stream_info,
            frame_index: 0,
            frame_duration_hns,
        })
    }

    fn encode(&mut self, frame_rgba: &[u8]) -> Result<Vec<u8>, HostRuntimeError> {
        use windows::Win32::Media::MediaFoundation::MFT_INPUT_STATUS_ACCEPT_DATA;

        let expected = (self.config.width * self.config.height * 4) as usize;
        if frame_rgba.len() != expected {
            return Err(HostRuntimeError::Runtime(format!(
                "invalid rgba length: expected {expected}, got {}",
                frame_rgba.len()
            )));
        }

        let input_status = unsafe { self.transform.GetInputStatus(self.input_stream_id) }
            .map_err(|err| HostRuntimeError::Runtime(format!("GetInputStatus failed: {err}")))?;
        if (input_status & MFT_INPUT_STATUS_ACCEPT_DATA.0 as u32) == 0 {
            return Err(HostRuntimeError::Runtime(
                "encoder input stream is not accepting data".to_string(),
            ));
        }

        let sample_time = i64::try_from(self.frame_index)
            .unwrap_or(i64::MAX)
            .saturating_mul(self.frame_duration_hns);
        let sample_duration = self.frame_duration_hns.max(1);
        let sample = create_nv12_input_sample(
            self.config.width,
            self.config.height,
            frame_rgba,
            sample_time,
            sample_duration,
        )?;

        unsafe {
            self.transform
                .ProcessInput(self.input_stream_id, &sample, 0)
                .map_err(|err| HostRuntimeError::Runtime(format!("ProcessInput failed: {err}")))?;
        }
        self.frame_index = self.frame_index.saturating_add(1);

        let mut output = Vec::new();
        for _ in 0..4 {
            match self.process_output_once()? {
                ProcessOutputState::NeedMoreInput => break,
                ProcessOutputState::Encoded {
                    annex_b_payload,
                    incomplete,
                } => {
                    if !annex_b_payload.is_empty() {
                        output.extend_from_slice(&annex_b_payload);
                    }
                    if !incomplete {
                        break;
                    }
                }
            }
        }

        Ok(output)
    }

    fn process_output_once(&mut self) -> Result<ProcessOutputState, HostRuntimeError> {
        use std::mem::ManuallyDrop;
        use windows::Win32::Media::MediaFoundation::{
            MFT_OUTPUT_DATA_BUFFER, MFT_OUTPUT_DATA_BUFFER_INCOMPLETE,
            MFT_OUTPUT_STREAM_CAN_PROVIDE_SAMPLES, MFT_OUTPUT_STREAM_PROVIDES_SAMPLES,
            MF_E_TRANSFORM_NEED_MORE_INPUT,
        };

        let flags = self.output_stream_info.dwFlags;
        let provides_samples = (flags & (MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32)) != 0
            || (flags & (MFT_OUTPUT_STREAM_CAN_PROVIDE_SAMPLES.0 as u32)) != 0;

        let preallocated_sample = if provides_samples {
            None
        } else {
            Some(create_empty_sample_with_buffer(
                self.output_stream_info.cbSize.max(1),
            )?)
        };

        let mut output_buffer = MFT_OUTPUT_DATA_BUFFER {
            dwStreamID: self.output_stream_id,
            pSample: ManuallyDrop::new(preallocated_sample),
            dwStatus: 0,
            pEvents: ManuallyDrop::new(None),
        };
        let mut status = 0u32;

        let process_result = unsafe {
            self.transform
                .ProcessOutput(0, std::slice::from_mut(&mut output_buffer), &mut status)
        };
        let sample = unsafe { ManuallyDrop::take(&mut output_buffer.pSample) };
        let _events = unsafe { ManuallyDrop::take(&mut output_buffer.pEvents) };

        match process_result {
            Ok(()) => {
                let Some(sample) = sample else {
                    return Ok(ProcessOutputState::NeedMoreInput);
                };

                let raw_payload = read_sample_bytes(&sample)?;
                let annex_b_payload = ensure_annex_b(&raw_payload)?;
                let incomplete =
                    (output_buffer.dwStatus & (MFT_OUTPUT_DATA_BUFFER_INCOMPLETE.0 as u32)) != 0;

                Ok(ProcessOutputState::Encoded {
                    annex_b_payload,
                    incomplete,
                })
            }
            Err(err) if err.code() == MF_E_TRANSFORM_NEED_MORE_INPUT => {
                Ok(ProcessOutputState::NeedMoreInput)
            }
            Err(err) => Err(HostRuntimeError::Runtime(format!(
                "ProcessOutput failed: {err}"
            ))),
        }
    }
}

#[cfg(target_os = "windows")]
enum ProcessOutputState {
    NeedMoreInput,
    Encoded {
        annex_b_payload: Vec<u8>,
        incomplete: bool,
    },
}

#[cfg(target_os = "windows")]
fn find_hardware_h264_encoder(
    vendor_filter: MfVendorFilter,
) -> Result<windows::Win32::Media::MediaFoundation::IMFTransform, HostRuntimeError> {
    use std::ptr;
    use windows::Win32::{
        Media::MediaFoundation::{
            IMFActivate, IMFTransform, MFMediaType_Video, MFTEnumEx, MFVideoFormat_H264,
            MFVideoFormat_NV12, MFT_CATEGORY_VIDEO_ENCODER, MFT_ENUM_FLAG_HARDWARE,
            MFT_ENUM_FLAG_SORTANDFILTER, MFT_REGISTER_TYPE_INFO,
        },
        System::Com::CoTaskMemFree,
    };

    let input = MFT_REGISTER_TYPE_INFO {
        guidMajorType: MFMediaType_Video,
        guidSubtype: MFVideoFormat_NV12,
    };
    let output = MFT_REGISTER_TYPE_INFO {
        guidMajorType: MFMediaType_Video,
        guidSubtype: MFVideoFormat_H264,
    };

    let mut activates_ptr: *mut Option<IMFActivate> = ptr::null_mut();
    let mut activate_count: u32 = 0;
    unsafe {
        MFTEnumEx(
            MFT_CATEGORY_VIDEO_ENCODER,
            MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER,
            Some(&input),
            Some(&output),
            &mut activates_ptr,
            &mut activate_count,
        )
        .map_err(|err| HostRuntimeError::Runtime(format!("MFTEnumEx failed: {err}")))?;
    }

    if activate_count == 0 || activates_ptr.is_null() {
        return Err(HostRuntimeError::Runtime(
            "no hardware H.264 MFT encoder found".to_string(),
        ));
    }

    let mut selected: Option<IMFActivate> = None;
    for idx in 0..activate_count as usize {
        let maybe_activate = unsafe { ptr::read(activates_ptr.add(idx)) };
        let Some(activate) = maybe_activate else {
            continue;
        };

        if matches_vendor_filter(&activate, vendor_filter) {
            selected = Some(activate);
            break;
        }
    }

    unsafe {
        CoTaskMemFree(Some(activates_ptr as _));
    }

    let activate = selected.ok_or_else(|| {
        HostRuntimeError::Runtime(match vendor_filter {
            MfVendorFilter::Any => "no matching hardware H.264 MFT encoder".to_string(),
            MfVendorFilter::Nvidia => "NVIDIA hardware H.264 MFT encoder not found".to_string(),
            MfVendorFilter::Intel => "Intel QSV hardware H.264 MFT encoder not found".to_string(),
        })
    })?;

    unsafe { activate.ActivateObject::<IMFTransform>() }.map_err(|err| {
        HostRuntimeError::Runtime(format!("ActivateObject(IMFTransform) failed: {err}"))
    })
}

#[cfg(target_os = "windows")]
fn matches_vendor_filter(
    activate: &windows::Win32::Media::MediaFoundation::IMFActivate,
    vendor_filter: MfVendorFilter,
) -> bool {
    if vendor_filter == MfVendorFilter::Any {
        return true;
    }

    let name = activate
        .friendly_name_lowercase()
        .or_else(|| activate.hardware_url_lowercase())
        .unwrap_or_default();

    match vendor_filter {
        MfVendorFilter::Any => true,
        MfVendorFilter::Nvidia => name.contains("nvidia") || name.contains("nvenc"),
        MfVendorFilter::Intel => {
            name.contains("intel") || name.contains("quick sync") || name.contains("qsv")
        }
    }
}

#[cfg(target_os = "windows")]
trait MfActivateExt {
    fn friendly_name_lowercase(&self) -> Option<String>;
    fn hardware_url_lowercase(&self) -> Option<String>;
}

#[cfg(target_os = "windows")]
impl MfActivateExt for windows::Win32::Media::MediaFoundation::IMFActivate {
    fn friendly_name_lowercase(&self) -> Option<String> {
        get_activate_string_attr(
            self,
            &windows::Win32::Media::MediaFoundation::MFT_FRIENDLY_NAME_Attribute,
        )
        .map(|v| v.to_lowercase())
    }

    fn hardware_url_lowercase(&self) -> Option<String> {
        get_activate_string_attr(
            self,
            &windows::Win32::Media::MediaFoundation::MFT_ENUM_HARDWARE_URL_Attribute,
        )
        .map(|v| v.to_lowercase())
    }
}

#[cfg(target_os = "windows")]
fn get_activate_string_attr(
    activate: &windows::Win32::Media::MediaFoundation::IMFActivate,
    key: &windows::core::GUID,
) -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::System::Com::CoTaskMemFree;

    let mut value = PWSTR::null();
    let mut len = 0u32;
    if unsafe { activate.GetAllocatedString(key, &mut value, &mut len) }.is_err() {
        return None;
    }
    if value.is_null() {
        return None;
    }

    let s = unsafe {
        let slice = std::slice::from_raw_parts(value.0, len as usize);
        let mut utf16 = slice.to_vec();
        while utf16.last().copied() == Some(0) {
            utf16.pop();
        }
        String::from_utf16_lossy(&utf16)
    };

    unsafe {
        CoTaskMemFree(Some(value.0 as _));
    }

    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(target_os = "windows")]
fn detect_stream_ids(
    transform: &windows::Win32::Media::MediaFoundation::IMFTransform,
) -> Result<(u32, u32), HostRuntimeError> {
    let mut input_stream_count = 0u32;
    let mut output_stream_count = 0u32;
    unsafe { transform.GetStreamCount(&mut input_stream_count, &mut output_stream_count) }
        .map_err(|err| HostRuntimeError::Runtime(format!("GetStreamCount failed: {err}")))?;

    if input_stream_count == 0 || output_stream_count == 0 {
        return Err(HostRuntimeError::Runtime(
            "unexpected MFT stream count (zero input/output)".to_string(),
        ));
    }

    let mut input_stream_id = 0u32;
    let mut output_stream_id = 0u32;
    let mut input_ids = vec![0u32; input_stream_count as usize];
    let mut output_ids = vec![0u32; output_stream_count as usize];

    // Many encoders return E_NOTIMPL and expect stream id 0.
    if unsafe { transform.GetStreamIDs(&mut input_ids, &mut output_ids) }.is_ok() {
        input_stream_id = input_ids[0];
        output_stream_id = output_ids[0];
    }

    Ok((input_stream_id, output_stream_id))
}

#[cfg(target_os = "windows")]
fn configure_encoder_types(
    transform: &windows::Win32::Media::MediaFoundation::IMFTransform,
    config: &EncoderConfig,
) -> Result<(), HostRuntimeError> {
    use windows::Win32::Media::MediaFoundation::{
        IMFMediaType, MFCreateMediaType, MFMediaType_Video, MFVideoFormat_H264, MFVideoFormat_NV12,
        MFVideoInterlace_Progressive, MF_E_NO_MORE_TYPES, MF_LOW_LATENCY, MF_MT_AVG_BITRATE,
        MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE, MF_MT_INTERLACE_MODE, MF_MT_MAJOR_TYPE,
        MF_MT_PIXEL_ASPECT_RATIO, MF_MT_SUBTYPE,
    };

    let (input_stream_id, output_stream_id) = detect_stream_ids(transform)?;

    let input_type = unsafe { MFCreateMediaType() }.map_err(|err| {
        HostRuntimeError::Runtime(format!("MFCreateMediaType(input) failed: {err}"))
    })?;
    set_guid_attr(&input_type, &MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
    set_guid_attr(&input_type, &MF_MT_SUBTYPE, &MFVideoFormat_NV12)?;
    set_u64_pair_attr(&input_type, &MF_MT_FRAME_SIZE, config.width, config.height)?;
    set_u64_pair_attr(&input_type, &MF_MT_FRAME_RATE, config.fps, 1)?;
    set_u64_pair_attr(&input_type, &MF_MT_PIXEL_ASPECT_RATIO, 1, 1)?;
    set_u32_attr(
        &input_type,
        &MF_MT_INTERLACE_MODE,
        MFVideoInterlace_Progressive.0 as u32,
    )?;

    unsafe { transform.SetInputType(input_stream_id, &input_type, 0) }
        .map_err(|err| HostRuntimeError::Runtime(format!("SetInputType failed: {err}")))?;

    let preferred_output_type = unsafe { MFCreateMediaType() }.map_err(|err| {
        HostRuntimeError::Runtime(format!("MFCreateMediaType(output) failed: {err}"))
    })?;
    set_guid_attr(
        &preferred_output_type,
        &MF_MT_MAJOR_TYPE,
        &MFMediaType_Video,
    )?;
    set_guid_attr(&preferred_output_type, &MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
    set_u32_attr(
        &preferred_output_type,
        &MF_MT_AVG_BITRATE,
        config.bitrate_kbps.saturating_mul(1000),
    )?;
    set_u64_pair_attr(
        &preferred_output_type,
        &MF_MT_FRAME_SIZE,
        config.width,
        config.height,
    )?;
    set_u64_pair_attr(&preferred_output_type, &MF_MT_FRAME_RATE, config.fps, 1)?;
    set_u64_pair_attr(&preferred_output_type, &MF_MT_PIXEL_ASPECT_RATIO, 1, 1)?;
    set_u32_attr(
        &preferred_output_type,
        &MF_MT_INTERLACE_MODE,
        MFVideoInterlace_Progressive.0 as u32,
    )?;

    let output_set_result =
        unsafe { transform.SetOutputType(output_stream_id, &preferred_output_type, 0) };
    if output_set_result.is_err() {
        let mut type_idx = 0u32;
        let mut set_ok = false;
        loop {
            let available_type: IMFMediaType =
                match unsafe { transform.GetOutputAvailableType(output_stream_id, type_idx) } {
                    Ok(v) => v,
                    Err(err) if err.code() == MF_E_NO_MORE_TYPES => break,
                    Err(err) => {
                        return Err(HostRuntimeError::Runtime(format!(
                            "GetOutputAvailableType failed: {err}"
                        )))
                    }
                };

            type_idx = type_idx.saturating_add(1);
            let subtype = unsafe { available_type.GetGUID(&MF_MT_SUBTYPE) };
            if subtype.ok() != Some(MFVideoFormat_H264) {
                continue;
            }

            let _ = set_u32_attr(
                &available_type,
                &MF_MT_AVG_BITRATE,
                config.bitrate_kbps.saturating_mul(1000),
            );
            let _ = set_u64_pair_attr(
                &available_type,
                &MF_MT_FRAME_SIZE,
                config.width,
                config.height,
            );
            let _ = set_u64_pair_attr(&available_type, &MF_MT_FRAME_RATE, config.fps, 1);
            let _ = set_u64_pair_attr(&available_type, &MF_MT_PIXEL_ASPECT_RATIO, 1, 1);
            let _ = set_u32_attr(
                &available_type,
                &MF_MT_INTERLACE_MODE,
                MFVideoInterlace_Progressive.0 as u32,
            );

            if unsafe { transform.SetOutputType(output_stream_id, &available_type, 0) }.is_ok() {
                set_ok = true;
                break;
            }
        }

        if !set_ok {
            return Err(HostRuntimeError::Runtime(
                "unable to set H.264 output type on selected MFT".to_string(),
            ));
        }
    }

    if let Ok(attrs) = unsafe { transform.GetAttributes() } {
        let _ = unsafe { attrs.SetUINT32(&MF_LOW_LATENCY, 1) };
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn set_guid_attr(
    attrs: &windows::Win32::Media::MediaFoundation::IMFAttributes,
    key: &windows::core::GUID,
    value: &windows::core::GUID,
) -> Result<(), HostRuntimeError> {
    unsafe { attrs.SetGUID(key, value) }
        .map_err(|err| HostRuntimeError::Runtime(format!("SetGUID failed: {err}")))
}

#[cfg(target_os = "windows")]
fn set_u32_attr(
    attrs: &windows::Win32::Media::MediaFoundation::IMFAttributes,
    key: &windows::core::GUID,
    value: u32,
) -> Result<(), HostRuntimeError> {
    unsafe { attrs.SetUINT32(key, value) }
        .map_err(|err| HostRuntimeError::Runtime(format!("SetUINT32 failed: {err}")))
}

#[cfg(target_os = "windows")]
fn set_u64_pair_attr(
    attrs: &windows::Win32::Media::MediaFoundation::IMFAttributes,
    key: &windows::core::GUID,
    high: u32,
    low: u32,
) -> Result<(), HostRuntimeError> {
    let packed = ((high as u64) << 32) | low as u64;
    unsafe { attrs.SetUINT64(key, packed) }
        .map_err(|err| HostRuntimeError::Runtime(format!("SetUINT64 failed: {err}")))
}

#[cfg(target_os = "windows")]
fn create_empty_sample_with_buffer(
    capacity_bytes: u32,
) -> Result<windows::Win32::Media::MediaFoundation::IMFSample, HostRuntimeError> {
    use windows::Win32::Media::MediaFoundation::{MFCreateMemoryBuffer, MFCreateSample};

    let sample = unsafe { MFCreateSample() }
        .map_err(|err| HostRuntimeError::Runtime(format!("MFCreateSample failed: {err}")))?;
    let buffer = unsafe { MFCreateMemoryBuffer(capacity_bytes.max(1)) }
        .map_err(|err| HostRuntimeError::Runtime(format!("MFCreateMemoryBuffer failed: {err}")))?;
    unsafe { sample.AddBuffer(&buffer) }
        .map_err(|err| HostRuntimeError::Runtime(format!("IMFSample::AddBuffer failed: {err}")))?;
    Ok(sample)
}

#[cfg(target_os = "windows")]
fn create_nv12_input_sample(
    width: u32,
    height: u32,
    frame_rgba: &[u8],
    sample_time_hns: i64,
    sample_duration_hns: i64,
) -> Result<windows::Win32::Media::MediaFoundation::IMFSample, HostRuntimeError> {
    use windows::Win32::Media::MediaFoundation::{MFCreateMemoryBuffer, MFCreateSample};

    let nv12 = rgba_to_nv12(width, height, frame_rgba)?;
    let sample = unsafe { MFCreateSample() }
        .map_err(|err| HostRuntimeError::Runtime(format!("MFCreateSample failed: {err}")))?;
    let buffer = unsafe { MFCreateMemoryBuffer(nv12.len() as u32) }
        .map_err(|err| HostRuntimeError::Runtime(format!("MFCreateMemoryBuffer failed: {err}")))?;

    let mut dst = std::ptr::null_mut::<u8>();
    let mut max_len = 0u32;
    let mut cur_len = 0u32;
    unsafe {
        buffer
            .Lock(&mut dst, Some(&mut max_len), Some(&mut cur_len))
            .map_err(|err| {
                HostRuntimeError::Runtime(format!("IMFMediaBuffer::Lock failed: {err}"))
            })?;
        std::ptr::copy_nonoverlapping(nv12.as_ptr(), dst, nv12.len());
        buffer.Unlock().map_err(|err| {
            HostRuntimeError::Runtime(format!("IMFMediaBuffer::Unlock failed: {err}"))
        })?;
        buffer
            .SetCurrentLength(nv12.len() as u32)
            .map_err(|err| HostRuntimeError::Runtime(format!("SetCurrentLength failed: {err}")))?;
        sample.AddBuffer(&buffer).map_err(|err| {
            HostRuntimeError::Runtime(format!("IMFSample::AddBuffer failed: {err}"))
        })?;
        sample
            .SetSampleTime(sample_time_hns)
            .map_err(|err| HostRuntimeError::Runtime(format!("SetSampleTime failed: {err}")))?;
        sample
            .SetSampleDuration(sample_duration_hns.max(1))
            .map_err(|err| HostRuntimeError::Runtime(format!("SetSampleDuration failed: {err}")))?;
    }

    Ok(sample)
}

#[cfg(target_os = "windows")]
fn read_sample_bytes(
    sample: &windows::Win32::Media::MediaFoundation::IMFSample,
) -> Result<Vec<u8>, HostRuntimeError> {
    let buffer = unsafe { sample.ConvertToContiguousBuffer() }.map_err(|err| {
        HostRuntimeError::Runtime(format!("ConvertToContiguousBuffer failed: {err}"))
    })?;
    let current_len = unsafe { buffer.GetCurrentLength() }
        .map_err(|err| HostRuntimeError::Runtime(format!("GetCurrentLength failed: {err}")))?;
    if current_len == 0 {
        return Ok(Vec::new());
    }

    let mut ptr = std::ptr::null_mut::<u8>();
    let mut max_len = 0u32;
    let mut cur_len = 0u32;
    unsafe {
        buffer
            .Lock(&mut ptr, Some(&mut max_len), Some(&mut cur_len))
            .map_err(|err| {
                HostRuntimeError::Runtime(format!("IMFMediaBuffer::Lock failed: {err}"))
            })?;
        let bytes = std::slice::from_raw_parts(ptr, cur_len as usize).to_vec();
        buffer.Unlock().map_err(|err| {
            HostRuntimeError::Runtime(format!("IMFMediaBuffer::Unlock failed: {err}"))
        })?;
        Ok(bytes)
    }
}

#[cfg(target_os = "windows")]
fn rgba_to_nv12(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, HostRuntimeError> {
    let w = width as usize;
    let h = height as usize;
    let expected = w * h * 4;
    if rgba.len() != expected {
        return Err(HostRuntimeError::Runtime(format!(
            "invalid rgba length for NV12 conversion: expected {expected}, got {}",
            rgba.len()
        )));
    }
    if width % 2 != 0 || height % 2 != 0 {
        return Err(HostRuntimeError::Runtime(
            "NV12 conversion requires even width and height".to_string(),
        ));
    }

    let y_plane_len = w * h;
    let uv_plane_len = w * h / 2;
    let mut nv12 = vec![0u8; y_plane_len + uv_plane_len];
    let (y_plane, uv_plane) = nv12.split_at_mut(y_plane_len);

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            let r = rgba[idx] as i32;
            let g = rgba[idx + 1] as i32;
            let b = rgba[idx + 2] as i32;

            let y_val = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
            y_plane[y * w + x] = y_val.clamp(0, 255) as u8;
        }
    }

    for y in (0..h).step_by(2) {
        for x in (0..w).step_by(2) {
            let mut u_acc = 0i32;
            let mut v_acc = 0i32;
            for dy in 0..2 {
                for dx in 0..2 {
                    let px = x + dx;
                    let py = y + dy;
                    let idx = (py * w + px) * 4;
                    let r = rgba[idx] as i32;
                    let g = rgba[idx + 1] as i32;
                    let b = rgba[idx + 2] as i32;

                    u_acc += ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
                    v_acc += ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
                }
            }
            let u = (u_acc / 4).clamp(0, 255) as u8;
            let v = (v_acc / 4).clamp(0, 255) as u8;
            let uv_idx = (y / 2) * w + x;
            uv_plane[uv_idx] = u;
            uv_plane[uv_idx + 1] = v;
        }
    }

    Ok(nv12)
}

#[cfg(target_os = "macos")]
struct MacosVideoToolboxEncoder {
    config: EncoderConfig,
    session: VTCompressionSessionRef,
    frame_index: i64,
    queue: std::sync::Arc<EncodedFrameQueue>,
}

#[cfg(target_os = "macos")]
impl MacosVideoToolboxEncoder {
    fn new(config: EncoderConfig) -> Result<Self, HostRuntimeError> {
        validate_config(&config)?;
        if config.width % 2 != 0 || config.height % 2 != 0 {
            return Err(HostRuntimeError::Runtime(
                "VideoToolbox H.264 encoder requires even width and height".to_string(),
            ));
        }

        let supported = unsafe { VTIsHardwareEncodeSupported(KCM_VIDEO_CODEC_TYPE_H264) };
        if !supported {
            return Err(HostRuntimeError::Runtime(
                "VideoToolbox H.264 hardware encode not supported".to_string(),
            ));
        }

        let queue = std::sync::Arc::new(EncodedFrameQueue::default());
        let callback_ref = std::sync::Arc::as_ptr(&queue) as *mut std::ffi::c_void;
        let mut session: VTCompressionSessionRef = std::ptr::null_mut();
        let status = unsafe {
            VTCompressionSessionCreate(
                std::ptr::null(),
                config.width as i32,
                config.height as i32,
                KCM_VIDEO_CODEC_TYPE_H264,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                Some(vt_output_callback),
                callback_ref,
                &mut session,
            )
        };
        vt_status(status, "VTCompressionSessionCreate")?;
        if session.is_null() {
            return Err(HostRuntimeError::Runtime(
                "VideoToolbox returned null compression session".to_string(),
            ));
        }

        set_vt_bool_property(session, unsafe { kVTCompressionPropertyKey_RealTime }, true)?;
        set_vt_bool_property(
            session,
            unsafe { kVTCompressionPropertyKey_AllowFrameReordering },
            false,
        )?;
        set_vt_i32_property(
            session,
            unsafe { kVTCompressionPropertyKey_ExpectedFrameRate },
            config.fps as i32,
        )?;
        set_vt_i32_property(
            session,
            unsafe { kVTCompressionPropertyKey_AverageBitRate },
            (config.bitrate_kbps.saturating_mul(1000)).min(i32::MAX as u32) as i32,
        )?;
        set_vt_i32_property(
            session,
            unsafe { kVTCompressionPropertyKey_MaxKeyFrameInterval },
            (config.fps.saturating_mul(2)).max(1) as i32,
        )?;
        set_vt_cf_property(
            session,
            unsafe { kVTCompressionPropertyKey_ProfileLevel },
            unsafe { kVTProfileLevel_H264_Baseline_AutoLevel },
        )?;

        vt_status(
            unsafe { VTCompressionSessionPrepareToEncodeFrames(session) },
            "VTCompressionSessionPrepareToEncodeFrames",
        )?;

        Ok(Self {
            config,
            session,
            frame_index: 0,
            queue,
        })
    }

    fn encode(&mut self, frame_rgba: &[u8]) -> Result<Vec<u8>, HostRuntimeError> {
        let expected = (self.config.width * self.config.height * 4) as usize;
        if frame_rgba.len() != expected {
            return Err(HostRuntimeError::Runtime(format!(
                "invalid rgba length: expected {expected}, got {}",
                frame_rgba.len()
            )));
        }

        let pixel_buffer =
            create_bgra_pixel_buffer(self.config.width, self.config.height, frame_rgba)?;
        let pts = CMTime {
            value: self.frame_index,
            timescale: self.config.fps as i32,
            flags: KCM_TIME_FLAGS_VALID,
            epoch: 0,
        };
        let duration = CMTime {
            value: 1,
            timescale: self.config.fps as i32,
            flags: KCM_TIME_FLAGS_VALID,
            epoch: 0,
        };

        vt_status(
            unsafe {
                VTCompressionSessionEncodeFrame(
                    self.session,
                    pixel_buffer,
                    pts,
                    duration,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            "VTCompressionSessionEncodeFrame",
        )?;
        let _ = unsafe { VTCompressionSessionCompleteFrames(self.session, KCM_TIME_INVALID) };

        unsafe { CFRelease(pixel_buffer as CFTypeRef) };
        self.frame_index = self.frame_index.saturating_add(1);

        self.queue.wait_for_frame()
    }
}

#[cfg(target_os = "macos")]
impl Drop for MacosVideoToolboxEncoder {
    fn drop(&mut self) {
        unsafe {
            let _ = VTCompressionSessionCompleteFrames(self.session, KCM_TIME_INVALID);
            VTCompressionSessionInvalidate(self.session);
            CFRelease(self.session as CFTypeRef);
        }
    }
}

#[cfg(target_os = "macos")]
#[derive(Default)]
struct EncodedFrameQueue {
    frames: std::sync::Mutex<std::collections::VecDeque<Vec<u8>>>,
    cv: std::sync::Condvar,
}

#[cfg(target_os = "macos")]
impl EncodedFrameQueue {
    fn push_frame(&self, frame: Vec<u8>) {
        if let Ok(mut guard) = self.frames.lock() {
            guard.push_back(frame);
            self.cv.notify_one();
        }
    }

    fn wait_for_frame(&self) -> Result<Vec<u8>, HostRuntimeError> {
        let timeout = std::time::Duration::from_millis(250);
        let mut guard = self
            .frames
            .lock()
            .map_err(|_| HostRuntimeError::Runtime("frame queue poisoned".to_string()))?;

        while guard.is_empty() {
            let (new_guard, wait_result) = self
                .cv
                .wait_timeout(guard, timeout)
                .map_err(|_| HostRuntimeError::Runtime("frame queue wait failed".to_string()))?;
            guard = new_guard;
            if wait_result.timed_out() && guard.is_empty() {
                return Err(HostRuntimeError::Runtime(
                    "VideoToolbox encode timeout waiting for output frame".to_string(),
                ));
            }
        }

        guard.pop_front().ok_or_else(|| {
            HostRuntimeError::Runtime("VideoToolbox output queue unexpectedly empty".to_string())
        })
    }
}

#[cfg(target_os = "macos")]
extern "C" fn vt_output_callback(
    output_callback_ref_con: *mut std::ffi::c_void,
    _source_frame_ref_con: *mut std::ffi::c_void,
    status: OSStatus,
    _info_flags: VTEncodeInfoFlags,
    sample_buffer: CMSampleBufferRef,
) {
    if output_callback_ref_con.is_null() || sample_buffer.is_null() || status != 0 {
        return;
    }
    if unsafe { !CMSampleBufferDataIsReady(sample_buffer) } {
        return;
    }

    let queue = unsafe { &*(output_callback_ref_con as *const EncodedFrameQueue) };
    if let Some(frame) = unsafe { extract_annex_b_from_sample_buffer(sample_buffer) } {
        if !frame.is_empty() {
            queue.push_frame(frame);
        }
    }
}

#[cfg(target_os = "macos")]
unsafe fn extract_annex_b_from_sample_buffer(sample_buffer: CMSampleBufferRef) -> Option<Vec<u8>> {
    let block = CMSampleBufferGetDataBuffer(sample_buffer);
    if block.is_null() {
        return None;
    }

    let mut length_at_offset = 0usize;
    let mut total_length = 0usize;
    let mut data_ptr = std::ptr::null_mut::<i8>();
    if CMBlockBufferGetDataPointer(
        block,
        0,
        &mut length_at_offset,
        &mut total_length,
        &mut data_ptr,
    ) != 0
    {
        return None;
    }
    if data_ptr.is_null() || total_length == 0 {
        return None;
    }

    let payload = std::slice::from_raw_parts(data_ptr as *const u8, total_length);
    let mut annex_b = ensure_annex_b(payload).ok()?;

    let format_desc = CMSampleBufferGetFormatDescription(sample_buffer);
    if !format_desc.is_null() && !contains_nal_type(&annex_b, 7) {
        if let Some((sps, pps)) = get_h264_parameter_sets(format_desc) {
            let mut with_ps = Vec::with_capacity(sps.len() + pps.len() + annex_b.len() + 12);
            with_ps.extend_from_slice(&[0, 0, 0, 1]);
            with_ps.extend_from_slice(&sps);
            with_ps.extend_from_slice(&[0, 0, 0, 1]);
            with_ps.extend_from_slice(&pps);
            with_ps.extend_from_slice(&annex_b);
            annex_b = with_ps;
        }
    }

    Some(annex_b)
}

#[cfg(target_os = "macos")]
fn contains_nal_type(payload: &[u8], nal_type: u8) -> bool {
    let mut i = 0usize;
    while i + 4 < payload.len() {
        if payload[i..].starts_with(&[0, 0, 0, 1]) {
            let hdr = payload[i + 4] & 0x1f;
            if hdr == nal_type {
                return true;
            }
            i += 4;
        } else if payload[i..].starts_with(&[0, 0, 1]) {
            let hdr = payload[i + 3] & 0x1f;
            if hdr == nal_type {
                return true;
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    false
}

#[cfg(target_os = "macos")]
fn get_h264_parameter_sets(format_desc: CMVideoFormatDescriptionRef) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut set_count = 0usize;
    let mut nal_len = 0i32;

    let mut sps_ptr = std::ptr::null::<u8>();
    let mut sps_len = 0usize;
    let status_sps = unsafe {
        CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
            format_desc,
            0,
            &mut sps_ptr,
            &mut sps_len,
            &mut set_count,
            &mut nal_len,
        )
    };
    if status_sps != 0 || sps_ptr.is_null() || sps_len == 0 {
        return None;
    }

    let mut pps_ptr = std::ptr::null::<u8>();
    let mut pps_len = 0usize;
    let status_pps = unsafe {
        CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
            format_desc,
            1,
            &mut pps_ptr,
            &mut pps_len,
            &mut set_count,
            &mut nal_len,
        )
    };
    if status_pps != 0 || pps_ptr.is_null() || pps_len == 0 {
        return None;
    }

    let sps = unsafe { std::slice::from_raw_parts(sps_ptr, sps_len).to_vec() };
    let pps = unsafe { std::slice::from_raw_parts(pps_ptr, pps_len).to_vec() };
    Some((sps, pps))
}

#[cfg(target_os = "macos")]
fn set_vt_bool_property(
    session: VTCompressionSessionRef,
    key: CFStringRef,
    value: bool,
) -> Result<(), HostRuntimeError> {
    let cf_bool = if value {
        unsafe { kCFBooleanTrue as CFTypeRef }
    } else {
        unsafe { kCFBooleanFalse as CFTypeRef }
    };
    set_vt_cf_property(session, key, cf_bool)
}

#[cfg(target_os = "macos")]
fn set_vt_i32_property(
    session: VTCompressionSessionRef,
    key: CFStringRef,
    value: i32,
) -> Result<(), HostRuntimeError> {
    let number = unsafe {
        CFNumberCreate(
            std::ptr::null(),
            KCF_NUMBER_SINT32_TYPE,
            (&value as *const i32).cast(),
        )
    };
    if number.is_null() {
        return Err(HostRuntimeError::Runtime(
            "CFNumberCreate returned null".to_string(),
        ));
    }
    let result = set_vt_cf_property(session, key, number as CFTypeRef);
    unsafe { CFRelease(number as CFTypeRef) };
    result
}

#[cfg(target_os = "macos")]
fn set_vt_cf_property(
    session: VTCompressionSessionRef,
    key: CFStringRef,
    value: CFTypeRef,
) -> Result<(), HostRuntimeError> {
    vt_status(
        unsafe { VTSessionSetProperty(session as VTSessionRef, key, value) },
        "VTSessionSetProperty",
    )
}

#[cfg(target_os = "macos")]
fn vt_status(status: OSStatus, ctx: &str) -> Result<(), HostRuntimeError> {
    if status == 0 {
        Ok(())
    } else {
        Err(HostRuntimeError::Runtime(format!(
            "{ctx} failed: OSStatus({status})"
        )))
    }
}

#[cfg(target_os = "macos")]
fn create_bgra_pixel_buffer(
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<CVPixelBufferRef, HostRuntimeError> {
    let mut pixel_buffer: CVPixelBufferRef = std::ptr::null_mut();
    vt_status(
        unsafe {
            CVPixelBufferCreate(
                std::ptr::null(),
                width as usize,
                height as usize,
                KCV_PIXEL_FORMAT_TYPE_32BGRA,
                std::ptr::null(),
                &mut pixel_buffer,
            )
        },
        "CVPixelBufferCreate",
    )?;
    if pixel_buffer.is_null() {
        return Err(HostRuntimeError::Runtime(
            "CVPixelBufferCreate returned null".to_string(),
        ));
    }

    vt_status(
        unsafe { CVPixelBufferLockBaseAddress(pixel_buffer, 0) },
        "CVPixelBufferLockBaseAddress",
    )?;

    let base = unsafe { CVPixelBufferGetBaseAddress(pixel_buffer) as *mut u8 };
    if base.is_null() {
        unsafe {
            let _ = CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);
            CFRelease(pixel_buffer as CFTypeRef);
        }
        return Err(HostRuntimeError::Runtime(
            "CVPixelBuffer base address is null".to_string(),
        ));
    }

    let stride = unsafe { CVPixelBufferGetBytesPerRow(pixel_buffer) };
    let width_usize = width as usize;
    let height_usize = height as usize;
    for row in 0..height_usize {
        let src_row = &rgba[row * width_usize * 4..(row + 1) * width_usize * 4];
        let dst_row =
            unsafe { std::slice::from_raw_parts_mut(base.add(row * stride), width_usize * 4) };
        for px in 0..width_usize {
            let si = px * 4;
            dst_row[si] = src_row[si + 2];
            dst_row[si + 1] = src_row[si + 1];
            dst_row[si + 2] = src_row[si];
            dst_row[si + 3] = src_row[si + 3];
        }
    }

    vt_status(
        unsafe { CVPixelBufferUnlockBaseAddress(pixel_buffer, 0) },
        "CVPixelBufferUnlockBaseAddress",
    )?;

    Ok(pixel_buffer)
}

#[cfg(target_os = "macos")]
type CFTypeRef = *const std::ffi::c_void;
#[cfg(target_os = "macos")]
type CFAllocatorRef = *const std::ffi::c_void;
#[cfg(target_os = "macos")]
type CFDictionaryRef = *const std::ffi::c_void;
#[cfg(target_os = "macos")]
type CFStringRef = *const std::ffi::c_void;
#[cfg(target_os = "macos")]
type CFBooleanRef = *const std::ffi::c_void;
#[cfg(target_os = "macos")]
type CFNumberRef = *const std::ffi::c_void;
#[cfg(target_os = "macos")]
type CVPixelBufferRef = *mut std::ffi::c_void;
#[cfg(target_os = "macos")]
type VTCompressionSessionRef = *mut std::ffi::c_void;
#[cfg(target_os = "macos")]
type VTSessionRef = *mut std::ffi::c_void;
#[cfg(target_os = "macos")]
type VTEncodeInfoFlags = u32;
#[cfg(target_os = "macos")]
type CMSampleBufferRef = *mut std::ffi::c_void;
#[cfg(target_os = "macos")]
type CMBlockBufferRef = *mut std::ffi::c_void;
#[cfg(target_os = "macos")]
type CMVideoFormatDescriptionRef = *const std::ffi::c_void;
#[cfg(target_os = "macos")]
type OSStatus = i32;

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CMTime {
    value: i64,
    timescale: i32,
    flags: u32,
    epoch: i64,
}

#[cfg(target_os = "macos")]
const KCM_VIDEO_CODEC_TYPE_H264: u32 = 0x6176_6331;
#[cfg(target_os = "macos")]
const KCM_TIME_FLAGS_VALID: u32 = 1;
#[cfg(target_os = "macos")]
const KCM_TIME_INVALID: CMTime = CMTime {
    value: 0,
    timescale: 0,
    flags: 0,
    epoch: 0,
};
#[cfg(target_os = "macos")]
const KCV_PIXEL_FORMAT_TYPE_32BGRA: u32 = 0x4247_5241;
#[cfg(target_os = "macos")]
const KCF_NUMBER_SINT32_TYPE: i32 = 3;

#[cfg(target_os = "macos")]
#[link(name = "VideoToolbox", kind = "framework")]
extern "C" {
    fn VTIsHardwareEncodeSupported(codec_type: u32) -> bool;
    fn VTCompressionSessionCreate(
        allocator: CFAllocatorRef,
        width: i32,
        height: i32,
        codec_type: u32,
        encoder_specification: CFDictionaryRef,
        source_image_buffer_attributes: CFDictionaryRef,
        compressed_data_allocator: CFAllocatorRef,
        output_callback: Option<
            extern "C" fn(
                *mut std::ffi::c_void,
                *mut std::ffi::c_void,
                OSStatus,
                VTEncodeInfoFlags,
                CMSampleBufferRef,
            ),
        >,
        output_callback_ref_con: *mut std::ffi::c_void,
        compression_session_out: *mut VTCompressionSessionRef,
    ) -> OSStatus;
    fn VTCompressionSessionPrepareToEncodeFrames(session: VTCompressionSessionRef) -> OSStatus;
    fn VTCompressionSessionEncodeFrame(
        session: VTCompressionSessionRef,
        image_buffer: CVPixelBufferRef,
        presentation_time_stamp: CMTime,
        duration: CMTime,
        frame_properties: CFDictionaryRef,
        source_frame_ref_con: *mut std::ffi::c_void,
        info_flags_out: *mut VTEncodeInfoFlags,
    ) -> OSStatus;
    fn VTCompressionSessionCompleteFrames(
        session: VTCompressionSessionRef,
        complete_until_presentation_time_stamp: CMTime,
    ) -> OSStatus;
    fn VTCompressionSessionInvalidate(session: VTCompressionSessionRef);
    fn VTSessionSetProperty(
        session: VTSessionRef,
        property_key: CFStringRef,
        property_value: CFTypeRef,
    ) -> OSStatus;

    static kVTCompressionPropertyKey_RealTime: CFStringRef;
    static kVTCompressionPropertyKey_ProfileLevel: CFStringRef;
    static kVTCompressionPropertyKey_AllowFrameReordering: CFStringRef;
    static kVTCompressionPropertyKey_ExpectedFrameRate: CFStringRef;
    static kVTCompressionPropertyKey_AverageBitRate: CFStringRef;
    static kVTCompressionPropertyKey_MaxKeyFrameInterval: CFStringRef;
    static kVTProfileLevel_H264_Baseline_AutoLevel: CFStringRef;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreVideo", kind = "framework")]
extern "C" {
    fn CVPixelBufferCreate(
        allocator: CFAllocatorRef,
        width: usize,
        height: usize,
        pixel_format_type: u32,
        pixel_buffer_attributes: CFDictionaryRef,
        pixel_buffer_out: *mut CVPixelBufferRef,
    ) -> OSStatus;
    fn CVPixelBufferLockBaseAddress(pixel_buffer: CVPixelBufferRef, lock_flags: u64) -> OSStatus;
    fn CVPixelBufferUnlockBaseAddress(pixel_buffer: CVPixelBufferRef, lock_flags: u64) -> OSStatus;
    fn CVPixelBufferGetBaseAddress(pixel_buffer: CVPixelBufferRef) -> *mut std::ffi::c_void;
    fn CVPixelBufferGetBytesPerRow(pixel_buffer: CVPixelBufferRef) -> usize;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreMedia", kind = "framework")]
extern "C" {
    fn CMSampleBufferDataIsReady(sample_buffer: CMSampleBufferRef) -> bool;
    fn CMSampleBufferGetDataBuffer(sample_buffer: CMSampleBufferRef) -> CMBlockBufferRef;
    fn CMSampleBufferGetFormatDescription(
        sample_buffer: CMSampleBufferRef,
    ) -> CMVideoFormatDescriptionRef;
    fn CMBlockBufferGetDataPointer(
        block_buffer: CMBlockBufferRef,
        offset_into_data: usize,
        length_at_offset_out: *mut usize,
        total_length_out: *mut usize,
        data_pointer_out: *mut *mut i8,
    ) -> OSStatus;
    fn CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
        video_desc: CMVideoFormatDescriptionRef,
        parameter_set_index: usize,
        parameter_set_pointer_out: *mut *const u8,
        parameter_set_size_out: *mut usize,
        parameter_set_count_out: *mut usize,
        nal_unit_header_length_out: *mut i32,
    ) -> OSStatus;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: CFTypeRef);
    fn CFNumberCreate(
        allocator: CFAllocatorRef,
        the_type: i32,
        value_ptr: *const std::ffi::c_void,
    ) -> CFNumberRef;
    static kCFBooleanTrue: CFBooleanRef;
    static kCFBooleanFalse: CFBooleanRef;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_encoder_config() {
        let cfg = EncoderConfig {
            width: 0,
            height: 1080,
            fps: 60,
            bitrate_kbps: 6000,
        };
        assert!(H264Encoder::new(cfg).is_err());
    }

    #[test]
    fn rejects_invalid_frame_size() {
        let cfg = EncoderConfig {
            width: 64,
            height: 64,
            fps: 30,
            bitrate_kbps: 1000,
        };
        let mut enc = H264Encoder::new(cfg).expect("encoder");
        assert!(enc.encode(&[1, 2, 3]).is_err());
    }

    #[test]
    fn software_payload_has_expected_length() {
        let cfg = EncoderConfig {
            width: 8,
            height: 8,
            fps: 30,
            bitrate_kbps: 1000,
        };

        std::env::set_var("XCONNECT_H264_BACKEND", "software");
        let mut enc = H264Encoder::new(cfg).expect("encoder");
        let frame = vec![255u8; 8 * 8 * 4];
        let payload = enc.encode(&frame).expect("encoded payload");
        assert_eq!(payload.len(), 8 * 8 * 3);
    }

    #[test]
    fn converts_avcc_to_annex_b() {
        // 00 00 00 04 [NAL] 00 00 00 03 [NAL]
        let avcc = [
            0u8, 0, 0, 4, 0x67, 0x64, 0, 0x1f, 0, 0, 0, 3, 0x68, 0xeb, 0xef,
        ];
        let annex_b = ensure_annex_b(&avcc).expect("annex-b");
        assert!(annex_b.starts_with(&[0, 0, 0, 1, 0x67]));
        assert!(annex_b.windows(5).any(|w| w == [0, 0, 0, 1, 0x68]));
    }
}
