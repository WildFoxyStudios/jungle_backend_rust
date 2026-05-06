use image::GenericImageView;
use image::imageops::FilterType;

pub struct ImageProcessResult {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: image::ImageFormat,
}

pub fn resize_image(
    data: &[u8],
    max_width: u32,
    max_height: u32,
) -> Result<ImageProcessResult, String> {
    let img = image::load_from_memory(data).map_err(|e| format!("Image decode error: {}", e))?;
    let (w, h) = img.dimensions();

    if w <= max_width && h <= max_height {
        return Ok(ImageProcessResult {
            data: data.to_vec(),
            width: w,
            height: h,
            format: image::ImageFormat::Jpeg,
        });
    }

    let resized = img.resize(max_width, max_height, FilterType::Lanczos3);
    let (nw, nh) = resized.dimensions();

    let mut buf = std::io::Cursor::new(Vec::new());
    resized
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Image encode error: {}", e))?;

    Ok(ImageProcessResult {
        data: buf.into_inner(),
        width: nw,
        height: nh,
        format: image::ImageFormat::Jpeg,
    })
}

pub fn generate_thumbnail(data: &[u8], size: u32) -> Result<ImageProcessResult, String> {
    let img = image::load_from_memory(data).map_err(|e| format!("Image decode error: {}", e))?;
    let thumb = img.thumbnail(size, size);
    let (w, h) = thumb.dimensions();

    let mut buf = std::io::Cursor::new(Vec::new());
    thumb
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Thumbnail encode error: {}", e))?;

    Ok(ImageProcessResult {
        data: buf.into_inner(),
        width: w,
        height: h,
        format: image::ImageFormat::Jpeg,
    })
}

pub fn crop_center(
    data: &[u8],
    target_width: u32,
    target_height: u32,
) -> Result<ImageProcessResult, String> {
    let img = image::load_from_memory(data).map_err(|e| format!("Image decode error: {}", e))?;
    let (w, h) = img.dimensions();

    let ratio_w = w as f64 / target_width as f64;
    let ratio_h = h as f64 / target_height as f64;
    let ratio = ratio_w.min(ratio_h);

    let nw = (target_width as f64 * ratio) as u32;
    let nh = (target_height as f64 * ratio) as u32;

    let x = (w - nw) / 2;
    let y = (h - nh) / 2;

    let cropped = img.crop_imm(x, y, nw, nh);
    let resized = cropped.resize_exact(target_width, target_height, FilterType::Lanczos3);

    let mut buf = std::io::Cursor::new(Vec::new());
    resized
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Crop encode error: {}", e))?;

    Ok(ImageProcessResult {
        data: buf.into_inner(),
        width: target_width,
        height: target_height,
        format: image::ImageFormat::Jpeg,
    })
}

pub fn get_image_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    image::load_from_memory(data)
        .ok()
        .map(|img| img.dimensions())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_image(width: u32, height: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
        buf.into_inner()
    }

    #[test]
    fn test_get_image_dimensions() {
        let data = create_test_image(320, 240);
        let dims = get_image_dimensions(&data);
        assert_eq!(dims, Some((320, 240)));
    }

    #[test]
    fn test_resize_image_smaller_passthrough() {
        let data = create_test_image(100, 100);
        let result = resize_image(&data, 2048, 2048).unwrap();
        assert!(result.width <= 2048);
        assert!(result.height <= 2048);
    }

    #[test]
    fn test_resize_image_downscales() {
        let data = create_test_image(4000, 3000);
        let result = resize_image(&data, 2048, 2048).unwrap();
        assert!(result.width <= 2048);
        assert!(result.height <= 2048);
    }

    #[test]
    fn test_generate_thumbnail() {
        let data = create_test_image(800, 600);
        let result = generate_thumbnail(&data, 200).unwrap();
        assert!(result.width <= 200);
        assert!(result.height <= 200);
        assert!(!result.data.is_empty());
    }

    #[test]
    fn test_crop_center() {
        let data = create_test_image(800, 600);
        let result = crop_center(&data, 200, 200).unwrap();
        assert_eq!(result.width, 200);
        assert_eq!(result.height, 200);
    }

    #[test]
    fn test_invalid_image_data() {
        let data = b"not an image";
        assert!(resize_image(data, 100, 100).is_err());
        assert!(generate_thumbnail(data, 100).is_err());
        assert_eq!(get_image_dimensions(data), None);
    }
}
