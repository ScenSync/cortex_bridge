//! ROS message to Rerun data converters

use crate::{RerunBridgeError, Result};

/// Convert ROS Image message (CDR format) to RGB8 data
pub fn parse_ros_image_cdr(cdr_data: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    // TODO: Implement proper CDR deserialization
    // For MVP, this is a placeholder
    
    // Simplified parsing (real implementation needs proper CDR decoder)
    if cdr_data.len() < 100 {
        return Err(RerunBridgeError::InvalidData("CDR data too small".to_string()));
    }
    
    // Placeholder: extract width, height, data
    // Real implementation would use ros2_rust or custom CDR parser
    let width = 640u32;
    let height = 480u32;
    let rgb_data = vec![0u8; (width * height * 3) as usize];
    
    Ok((width, height, rgb_data))
}

/// Convert ROS PointCloud2 message (CDR format) to point data
pub fn parse_ros_pointcloud2_cdr(cdr_data: &[u8]) -> Result<(Vec<f32>, Vec<u8>)> {
    // TODO: Implement proper CDR deserialization
    
    if cdr_data.len() < 100 {
        return Err(RerunBridgeError::InvalidData("CDR data too small".to_string()));
    }
    
    // Placeholder
    let points = vec![0.0f32; 300]; // 100 points × 3 coordinates
    let colors = vec![255u8; 300];  // 100 points × 3 RGB values
    
    Ok((points, colors))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image() {
        let dummy_cdr = vec![0u8; 1000];
        let result = parse_ros_image_cdr(&dummy_cdr);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_pointcloud() {
        let dummy_cdr = vec![0u8; 1000];
        let result = parse_ros_pointcloud2_cdr(&dummy_cdr);
        assert!(result.is_ok());
    }
}

