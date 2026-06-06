pub(super) fn hex_to_rgba(hex: &str) -> gpui::Rgba {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    let val = u32::from_str_radix(h, 16).unwrap_or(0x2563EB);
    gpui::rgba((val << 8) | 0xFF)
}

pub(super) fn rgba_with_alpha(hex: &str, alpha: f32) -> gpui::Rgba {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    let val = u32::from_str_radix(h, 16).unwrap_or(0x2563EB);
    let a = (alpha * 255.0) as u32;
    gpui::rgba((val << 8) | a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_rgba_with_hash() {
        let color = hex_to_rgba("#ff0000");
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn hex_to_rgba_without_hash() {
        let color = hex_to_rgba("00ff00");
        assert_eq!(color.r, 0.0);
        assert_eq!(color.g, 1.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn hex_to_rgba_blue() {
        let color = hex_to_rgba("#0000ff");
        assert_eq!(color.r, 0.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 1.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn hex_to_rgba_mixed_color() {
        let color = hex_to_rgba("#2563eb");
        assert!((color.r - 0.145).abs() < 0.01);
        assert!((color.g - 0.388).abs() < 0.01);
        assert!((color.b - 0.922).abs() < 0.01);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn hex_to_rgba_invalid_defaults() {
        let color = hex_to_rgba("invalid");
        let default = hex_to_rgba("#2563eb");
        assert_eq!(color.r, default.r);
        assert_eq!(color.g, default.g);
        assert_eq!(color.b, default.b);
    }

    #[test]
    fn rgba_with_alpha_full() {
        let color = rgba_with_alpha("#ff0000", 1.0);
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn rgba_with_alpha_half() {
        let color = rgba_with_alpha("#ff0000", 0.5);
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
        assert!((color.a - 0.5).abs() < 0.01);
    }

    #[test]
    fn rgba_with_alpha_quarter() {
        let color = rgba_with_alpha("#00ff00", 0.25);
        assert_eq!(color.r, 0.0);
        assert_eq!(color.g, 1.0);
        assert_eq!(color.b, 0.0);
        assert!((color.a - 0.25).abs() < 0.01);
    }

    #[test]
    fn rgba_with_alpha_zero() {
        let color = rgba_with_alpha("#0000ff", 0.0);
        assert_eq!(color.r, 0.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 1.0);
        assert_eq!(color.a, 0.0);
    }
}
