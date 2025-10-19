//! ROS message to Rerun data converters

use crate::{RerunBridgeError, Result};

/// CDR Reader utility for parsing ROS messages
struct CdrReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> CdrReader<'a> {
    fn new(data: &'a [u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(RerunBridgeError::InvalidData("CDR data too small".to_string()));
        }
        // Skip CDR header (4 bytes: encapsulation kind + options)
        Ok(Self { data, pos: 4 })
    }

    fn align(&mut self, alignment: usize) {
        let remainder = self.pos % alignment;
        if remainder != 0 {
            self.pos += alignment - remainder;
        }
    }

    fn read_u32(&mut self) -> Result<u32> {
        self.align(4);
        if self.pos + 4 > self.data.len() {
            return Err(RerunBridgeError::InvalidData("Unexpected end of CDR data".to_string()));
        }
        let value = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8> {
        if self.pos >= self.data.len() {
            return Err(RerunBridgeError::InvalidData("Unexpected end of CDR data".to_string()));
        }
        let value = self.data[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn read_string(&mut self) -> Result<String> {
        let len = self.read_u32()? as usize;
        if len == 0 {
            return Ok(String::new());
        }
        
        // String length includes null terminator
        if self.pos + len > self.data.len() {
            return Err(RerunBridgeError::InvalidData("String extends beyond buffer".to_string()));
        }
        
        let string_bytes = &self.data[self.pos..self.pos + len - 1]; // Exclude null terminator
        self.pos += len;
        
        String::from_utf8(string_bytes.to_vec())
            .map_err(|e| RerunBridgeError::InvalidData(format!("Invalid UTF-8 in string: {}", e)))
    }

    fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>> {
        if self.pos + count > self.data.len() {
            return Err(RerunBridgeError::InvalidData("Byte array extends beyond buffer".to_string()));
        }
        let bytes = self.data[self.pos..self.pos + count].to_vec();
        self.pos += count;
        Ok(bytes)
    }

    fn read_sequence_length(&mut self) -> Result<u32> {
        self.read_u32()
    }
}

/// Convert ROS Image message (CDR format) to RGB8 data
/// 
/// ROS2 sensor_msgs/msg/Image structure:
/// - std_msgs/Header header
/// - uint32 height
/// - uint32 width  
/// - string encoding (e.g., "rgb8", "bgr8", "mono8")
/// - uint8 is_bigendian
/// - uint32 step (row stride in bytes)
/// - uint8[] data
pub fn parse_ros_image_cdr(cdr_data: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    let mut reader = CdrReader::new(cdr_data)?;
    
    // Parse Header (std_msgs/Header)
    // - stamp (builtin_interfaces/Time: int32 sec, uint32 nanosec)
    let _stamp_sec = reader.read_u32()?;
    let _stamp_nanosec = reader.read_u32()?;
    // - frame_id (string)
    let _frame_id = reader.read_string()?;
    
    // Parse Image fields
    let height = reader.read_u32()?;
    let width = reader.read_u32()?;
    let encoding = reader.read_string()?;
    let _is_bigendian = reader.read_u8()?;
    let step = reader.read_u32()?;
    
    // Read image data
    let data_len = reader.read_sequence_length()? as usize;
    let image_data = reader.read_bytes(data_len)?;
    
    crate::debug!(
        "📸 Parsed ROS Image: {}x{}, encoding={}, step={}, data_len={}",
        width, height, encoding, step, data_len
    );
    
    // Convert to RGB8 format if needed
    let rgb_data = match encoding.as_str() {
        "rgb8" => {
            // Already RGB8, use as-is
            image_data
        }
        "bgr8" => {
            // Convert BGR to RGB by swapping R and B channels
            convert_bgr8_to_rgb8(&image_data)
        }
        "mono8" | "8UC1" => {
            // Convert grayscale to RGB by repeating the value
            convert_mono8_to_rgb8(&image_data)
        }
        "rgba8" => {
            // Convert RGBA to RGB by dropping alpha channel
            convert_rgba8_to_rgb8(&image_data)
        }
        "bgra8" => {
            // Convert BGRA to RGB
            convert_bgra8_to_rgb8(&image_data)
        }
        _ => {
            crate::warn!("⚠️  Unsupported image encoding: {}, attempting to use as-is", encoding);
            image_data
        }
    };
    
    Ok((width, height, rgb_data))
}

/// Convert BGR8 to RGB8 by swapping R and B channels
fn convert_bgr8_to_rgb8(bgr_data: &[u8]) -> Vec<u8> {
    let mut rgb_data = Vec::with_capacity(bgr_data.len());
    for chunk in bgr_data.chunks_exact(3) {
        rgb_data.push(chunk[2]); // R
        rgb_data.push(chunk[1]); // G
        rgb_data.push(chunk[0]); // B
    }
    rgb_data
}

/// Convert MONO8 to RGB8 by replicating grayscale value
fn convert_mono8_to_rgb8(mono_data: &[u8]) -> Vec<u8> {
    let mut rgb_data = Vec::with_capacity(mono_data.len() * 3);
    for &gray in mono_data {
        rgb_data.push(gray);
        rgb_data.push(gray);
        rgb_data.push(gray);
    }
    rgb_data
}

/// Convert RGBA8 to RGB8 by dropping alpha channel
fn convert_rgba8_to_rgb8(rgba_data: &[u8]) -> Vec<u8> {
    let mut rgb_data = Vec::with_capacity((rgba_data.len() / 4) * 3);
    for chunk in rgba_data.chunks_exact(4) {
        rgb_data.push(chunk[0]); // R
        rgb_data.push(chunk[1]); // G
        rgb_data.push(chunk[2]); // B
        // Skip chunk[3] (alpha)
    }
    rgb_data
}

/// Convert BGRA8 to RGB8
fn convert_bgra8_to_rgb8(bgra_data: &[u8]) -> Vec<u8> {
    let mut rgb_data = Vec::with_capacity((bgra_data.len() / 4) * 3);
    for chunk in bgra_data.chunks_exact(4) {
        rgb_data.push(chunk[2]); // R
        rgb_data.push(chunk[1]); // G
        rgb_data.push(chunk[0]); // B
        // Skip chunk[3] (alpha)
    }
    rgb_data
}

/// PointField datatype constants
#[allow(dead_code)]
mod point_field_datatypes {
    pub const INT8: u8 = 1;
    pub const UINT8: u8 = 2;
    pub const INT16: u8 = 3;
    pub const UINT16: u8 = 4;
    pub const INT32: u8 = 5;
    pub const UINT32: u8 = 6;
    pub const FLOAT32: u8 = 7;
    pub const FLOAT64: u8 = 8;
}

/// PointField descriptor
#[derive(Debug)]
struct PointField {
    name: String,
    offset: u32,
    #[allow(dead_code)] // May be used for validation in the future
    datatype: u8,
    #[allow(dead_code)] // May be used for multi-element fields
    count: u32,
}

impl PointField {
    fn parse(reader: &mut CdrReader) -> Result<Self> {
        Ok(Self {
            name: reader.read_string()?,
            offset: reader.read_u32()?,
            datatype: reader.read_u8()?,
            count: reader.read_u32()?,
        })
    }
}

/// Convert ROS PointCloud2 message (CDR format) to point data
///
/// ROS2 sensor_msgs/msg/PointCloud2 structure:
/// - std_msgs/Header header
/// - uint32 height (1 for unordered cloud)
/// - uint32 width (number of points)
/// - PointField[] fields (data layout)
/// - bool is_bigendian
/// - uint32 point_step (bytes per point)
/// - uint32 row_step (width × point_step)
/// - uint8[] data
/// - bool is_dense
pub fn parse_ros_pointcloud2_cdr(cdr_data: &[u8]) -> Result<(Vec<f32>, Vec<u8>)> {
    let mut reader = CdrReader::new(cdr_data)?;
    
    // Parse Header
    let _stamp_sec = reader.read_u32()?;
    let _stamp_nanosec = reader.read_u32()?;
    let _frame_id = reader.read_string()?;
    
    // Parse PointCloud2 fields
    let height = reader.read_u32()?;
    let width = reader.read_u32()?;
    
    // Parse PointField array
    let num_fields = reader.read_sequence_length()? as usize;
    let mut fields = Vec::with_capacity(num_fields);
    for _ in 0..num_fields {
        fields.push(PointField::parse(&mut reader)?);
    }
    
    let _is_bigendian = reader.read_u8()?;
    let point_step = reader.read_u32()? as usize;
    let _row_step = reader.read_u32()?;
    
    // Read point data
    let data_len = reader.read_sequence_length()? as usize;
    let point_data = reader.read_bytes(data_len)?;
    
    let _is_dense = reader.read_u8()?;
    
    let num_points = (width * height) as usize;
    
    crate::debug!(
        "☁️  Parsed PointCloud2: {} points, {} fields, point_step={}, data_len={}",
        num_points, num_fields, point_step, data_len
    );
    
    // Find X, Y, Z, RGB field offsets
    let mut x_offset = None;
    let mut y_offset = None;
    let mut z_offset = None;
    let mut rgb_offset = None;
    
    for field in &fields {
        match field.name.as_str() {
            "x" => x_offset = Some(field.offset as usize),
            "y" => y_offset = Some(field.offset as usize),
            "z" => z_offset = Some(field.offset as usize),
            "rgb" | "rgba" => rgb_offset = Some(field.offset as usize),
            _ => {}
        }
    }
    
    // Validate required fields
    let x_off = x_offset.ok_or_else(|| RerunBridgeError::InvalidData("Missing 'x' field".to_string()))?;
    let y_off = y_offset.ok_or_else(|| RerunBridgeError::InvalidData("Missing 'y' field".to_string()))?;
    let z_off = z_offset.ok_or_else(|| RerunBridgeError::InvalidData("Missing 'z' field".to_string()))?;
    
    // Extract points (XYZ as flat array)
    let mut points = Vec::with_capacity(num_points * 3);
    let mut colors = Vec::with_capacity(num_points * 3);
    
    for i in 0..num_points {
        let point_start = i * point_step;
        
        if point_start + point_step > point_data.len() {
            break; // Truncated data
        }
        
        // Extract XYZ (assume FLOAT32)
        let x = read_f32(&point_data[point_start + x_off..]);
        let y = read_f32(&point_data[point_start + y_off..]);
        let z = read_f32(&point_data[point_start + z_off..]);
        
        // Skip NaN/Inf points
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            continue;
        }
        
        points.push(x);
        points.push(y);
        points.push(z);
        
        // Extract RGB if available
        if let Some(rgb_off) = rgb_offset {
            // RGB is typically packed as uint32 (0xRRGGBB)
            let rgb_packed = read_u32(&point_data[point_start + rgb_off..]);
            let r = ((rgb_packed >> 16) & 0xFF) as u8;
            let g = ((rgb_packed >> 8) & 0xFF) as u8;
            let b = (rgb_packed & 0xFF) as u8;
            
            colors.push(r);
            colors.push(g);
            colors.push(b);
        } else {
            // Default white color
            colors.push(255);
            colors.push(255);
            colors.push(255);
        }
    }
    
    crate::debug!("☁️  Extracted {} valid points from PointCloud2", points.len() / 3);
    
    Ok((points, colors))
}

/// Read f32 from byte slice (little-endian)
fn read_f32(bytes: &[u8]) -> f32 {
    if bytes.len() < 4 {
        return 0.0;
    }
    f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Read u32 from byte slice (little-endian)
fn read_u32(bytes: &[u8]) -> u32 {
    if bytes.len() < 4 {
        return 0;
    }
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

// ============================================================================
// Transform (TF/TF2) Parsing
// ============================================================================

/// Transform data for Rerun visualization
#[derive(Debug)]
pub struct Transform {
    pub frame_id: String,
    pub child_frame_id: String,
    pub timestamp_sec: u32,
    pub timestamp_nanosec: u32,
    pub translation: [f64; 3], // x, y, z
    pub rotation: [f64; 4],    // x, y, z, w (quaternion)
}

/// Parse ROS TransformStamped message (geometry_msgs/msg/TransformStamped)
///
/// Structure:
/// - std_msgs/Header header
/// - string child_frame_id
/// - geometry_msgs/Transform transform
///   - Vector3 translation (x, y, z)
///   - Quaternion rotation (x, y, z, w)
pub fn parse_ros_transform_cdr(cdr_data: &[u8]) -> Result<Transform> {
    let mut reader = CdrReader::new(cdr_data)?;
    
    // Parse Header
    let stamp_sec = reader.read_u32()?;
    let stamp_nanosec = reader.read_u32()?;
    let frame_id = reader.read_string()?;
    
    // Parse child_frame_id
    let child_frame_id = reader.read_string()?;
    
    // Parse Transform - Translation (Vector3)
    reader.align(8); // doubles need 8-byte alignment
    let tx = reader.read_f64()?;
    let ty = reader.read_f64()?;
    let tz = reader.read_f64()?;
    
    // Parse Transform - Rotation (Quaternion)
    let qx = reader.read_f64()?;
    let qy = reader.read_f64()?;
    let qz = reader.read_f64()?;
    let qw = reader.read_f64()?;
    
    crate::debug!(
        "🔄 Parsed Transform: {} -> {}, translation=[{:.3}, {:.3}, {:.3}]",
        frame_id, child_frame_id, tx, ty, tz
    );
    
    Ok(Transform {
        frame_id,
        child_frame_id,
        timestamp_sec: stamp_sec,
        timestamp_nanosec: stamp_nanosec,
        translation: [tx, ty, tz],
        rotation: [qx, qy, qz, qw],
    })
}

impl CdrReader<'_> {
    fn read_f64(&mut self) -> Result<f64> {
        self.align(8); // f64 requires 8-byte alignment
        if self.pos + 8 > self.data.len() {
            return Err(RerunBridgeError::InvalidData("Unexpected end of CDR data".to_string()));
        }
        let value = f64::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ]);
        self.pos += 8;
        Ok(value)
    }
}

// ============================================================================
// JointState Parsing
// ============================================================================

/// JointState data for robot visualization
#[derive(Debug)]
pub struct JointState {
    pub timestamp_sec: u32,
    pub timestamp_nanosec: u32,
    pub names: Vec<String>,
    pub positions: Vec<f64>,
    pub velocities: Vec<f64>,
    pub efforts: Vec<f64>,
}

/// Parse ROS JointState message (sensor_msgs/msg/JointState)
///
/// Structure:
/// - std_msgs/Header header
/// - string[] name
/// - float64[] position
/// - float64[] velocity
/// - float64[] effort
pub fn parse_ros_joint_state_cdr(cdr_data: &[u8]) -> Result<JointState> {
    let mut reader = CdrReader::new(cdr_data)?;
    
    // Parse Header
    let stamp_sec = reader.read_u32()?;
    let stamp_nanosec = reader.read_u32()?;
    let _frame_id = reader.read_string()?;
    
    // Parse joint names
    let num_names = reader.read_sequence_length()? as usize;
    let mut names = Vec::with_capacity(num_names);
    for _ in 0..num_names {
        names.push(reader.read_string()?);
    }
    
    // Parse positions
    let num_positions = reader.read_sequence_length()? as usize;
    let mut positions = Vec::with_capacity(num_positions);
    for _ in 0..num_positions {
        positions.push(reader.read_f64()?);
    }
    
    // Parse velocities
    let num_velocities = reader.read_sequence_length()? as usize;
    let mut velocities = Vec::with_capacity(num_velocities);
    for _ in 0..num_velocities {
        velocities.push(reader.read_f64()?);
    }
    
    // Parse efforts
    let num_efforts = reader.read_sequence_length()? as usize;
    let mut efforts = Vec::with_capacity(num_efforts);
    for _ in 0..num_efforts {
        efforts.push(reader.read_f64()?);
    }
    
    crate::debug!(
        "🤖 Parsed JointState: {} joints, pos={}, vel={}, eff={}",
        names.len(), positions.len(), velocities.len(), efforts.len()
    );
    
    Ok(JointState {
        timestamp_sec: stamp_sec,
        timestamp_nanosec: stamp_nanosec,
        names,
        positions,
        velocities,
        efforts,
    })
}

// ============================================================================
// Log Message Parsing
// ============================================================================

/// ROS log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LogLevel {
    Debug = 10,
    Info = 20,
    Warn = 30,
    Error = 40,
    Fatal = 50,
}

impl LogLevel {
    fn from_u8(value: u8) -> Self {
        match value {
            10 => LogLevel::Debug,
            20 => LogLevel::Info,
            30 => LogLevel::Warn,
            40 => LogLevel::Error,
            50 => LogLevel::Fatal,
            _ => LogLevel::Info, // Default to Info
        }
    }
}

/// ROS Log message
#[derive(Debug)]
pub struct LogMessage {
    pub timestamp_sec: u32,
    pub timestamp_nanosec: u32,
    pub level: LogLevel,
    pub name: String,
    pub msg: String,
    pub file: String,
    pub function: String,
    pub line: u32,
}

/// Parse ROS Log message (rcl_interfaces/msg/Log)
///
/// Structure:
/// - builtin_interfaces/Time stamp
/// - uint8 level
/// - string name
/// - string msg
/// - string file
/// - string function
/// - uint32 line
pub fn parse_ros_log_cdr(cdr_data: &[u8]) -> Result<LogMessage> {
    let mut reader = CdrReader::new(cdr_data)?;
    
    // Parse timestamp
    let stamp_sec = reader.read_u32()?;
    let stamp_nanosec = reader.read_u32()?;
    
    // Parse level
    let level = LogLevel::from_u8(reader.read_u8()?);
    
    // Parse strings
    let name = reader.read_string()?;
    let msg = reader.read_string()?;
    let file = reader.read_string()?;
    let function = reader.read_string()?;
    let line = reader.read_u32()?;
    
    crate::trace!(
        "📝 Parsed Log [{:?}] {}: {}",
        level, name, msg
    );
    
    Ok(LogMessage {
        timestamp_sec: stamp_sec,
        timestamp_nanosec: stamp_nanosec,
        level,
        name,
        msg,
        file,
        function,
        line,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_invalid_data() {
        // Test with data that's too small
        let dummy_cdr = vec![0u8; 10];
        let result = parse_ros_image_cdr(&dummy_cdr);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_pointcloud_invalid_data() {
        // Test with data that's too small
        let dummy_cdr = vec![0u8; 10];
        let result = parse_ros_pointcloud2_cdr(&dummy_cdr);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_color_conversions() {
        // Test BGR to RGB conversion
        let bgr_data = vec![255, 128, 64, 0, 255, 128]; // Two pixels
        let rgb_data = convert_bgr8_to_rgb8(&bgr_data);
        assert_eq!(rgb_data, vec![64, 128, 255, 128, 255, 0]);
        
        // Test MONO to RGB conversion
        let mono_data = vec![100, 200];
        let rgb_data = convert_mono8_to_rgb8(&mono_data);
        assert_eq!(rgb_data, vec![100, 100, 100, 200, 200, 200]);
        
        // Test RGBA to RGB conversion
        let rgba_data = vec![255, 128, 64, 255, 0, 255, 128, 128];
        let rgb_data = convert_rgba8_to_rgb8(&rgba_data);
        assert_eq!(rgb_data, vec![255, 128, 64, 0, 255, 128]);
    }
    
    #[test]
    fn test_log_level_conversion() {
        assert_eq!(LogLevel::from_u8(10), LogLevel::Debug);
        assert_eq!(LogLevel::from_u8(20), LogLevel::Info);
        assert_eq!(LogLevel::from_u8(30), LogLevel::Warn);
        assert_eq!(LogLevel::from_u8(40), LogLevel::Error);
        assert_eq!(LogLevel::from_u8(50), LogLevel::Fatal);
        assert_eq!(LogLevel::from_u8(99), LogLevel::Info); // Unknown defaults to Info
    }
    
    #[test]
    fn test_parse_transform_invalid_data() {
        let dummy_cdr = vec![0u8; 10];
        let result = parse_ros_transform_cdr(&dummy_cdr);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_parse_joint_state_invalid_data() {
        let dummy_cdr = vec![0u8; 10];
        let result = parse_ros_joint_state_cdr(&dummy_cdr);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_parse_log_invalid_data() {
        let dummy_cdr = vec![0u8; 10];
        let result = parse_ros_log_cdr(&dummy_cdr);
        assert!(result.is_err());
    }
}
