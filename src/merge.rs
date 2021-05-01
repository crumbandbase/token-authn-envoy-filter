/// A trait for values that can be merged.
///
/// # Examples
///
/// Implementing `Merge` for a struct:
///
/// ```
/// use merge::Merge;
///
/// struct Configuration {
///     name: String
///     age: Option<u64>,
/// }
///
/// let configuration = Configuration {
///     name: "Jean-Luc Picard".to_string(),
///     age: Some(59),
/// };
///
/// assert_eq!(
///     configuration.merge(Configuration {
///         name: "James T. Kirk".to_string(),
///         age: None,
///     }),
///     Configuration {
///         name: "James T. Kirk".to_string(),
///         age: Some(59),
///     },
/// );
/// ```
pub trait Merge {
    /// Merge another value into this value.
    fn merge(&self, other: Self) -> Self;
}

/// Implemenation of the `Merge` trait for the `Option` type. Fields with
/// `Some` value in the value being merged take precedence over value in the
/// value being merged into.
impl<T: Clone> Merge for Option<T> {
    fn merge(&self, mut other: Self) -> Self {
        if other.is_some() {
            return other.take();
        }
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::Merge;

    #[derive(Debug, PartialEq)]
    struct Configuration {
        name: String,
        age: Option<u64>,
    }

    impl Merge for Configuration {
        fn merge(&self, other: Configuration) -> Configuration {
            Configuration {
                name: other.name.clone(),
                age: self.age.merge(other.age),
            }
        }
    }

    #[test]
    fn merges_values() {
        let configuration = Configuration {
            name: "Jean-Luc Picard".to_string(),
            age: Some(59),
        };

        assert_eq!(
            configuration.merge(Configuration {
                name: "James T. Kirk".to_string(),
                age: None,
            }),
            Configuration {
                name: "James T. Kirk".to_string(),
                age: Some(59),
            },
        );
    }
}
