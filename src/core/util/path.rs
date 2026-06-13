//! 路径规范化（spec §4.7）
//!
//! 行为：
//! - 折叠 `//` 为 `/`
//! - 解析 `.` 与 `..`
//! - 不允许逃出根（`..` 在根位置时直接忽略）
//! - 保留前导 `/`

/// 规范化路径：处理 `.` `..` 与连续 `/`
///
/// # Examples
/// ```
/// use rapidgate::core::util::path::normalize;
/// assert_eq!(normalize("/a/./b/../c"), "/a/c");
/// assert_eq!(normalize("//a//b/"), "/a/b");
/// assert_eq!(normalize("/../../etc"), "/etc");
/// ```
pub fn normalize(input: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let leading_slash = input.starts_with('/');

    for seg in input.split('/') {
        match seg {
            "" | "." => continue,
            ".." => {
                if !out.is_empty() {
                    out.pop();
                }
            }
            other => out.push(other),
        }
    }

    let mut result = String::new();
    if leading_slash {
        result.push('/');
    }
    result.push_str(&out.join("/"));
    if result.is_empty() {
        return "/".to_string();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_double_slashes() {
        assert_eq!(normalize("/a//b"), "/a/b");
    }

    #[test]
    fn resolves_dot_segments() {
        assert_eq!(normalize("/a/./b"), "/a/b");
    }

    #[test]
    fn resolves_parent_segments() {
        assert_eq!(normalize("/a/b/../c"), "/a/c");
    }

    #[test]
    fn clamps_at_root() {
        assert_eq!(normalize("/../../etc"), "/etc");
    }

    #[test]
    fn strips_trailing_slash() {
        assert_eq!(normalize("/a/b/"), "/a/b");
    }
}
