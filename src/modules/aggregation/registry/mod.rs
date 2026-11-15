mod annotation;
mod declarations;
mod identity;
mod resolution;
mod token;

#[cfg(test)]
mod tests;

pub(crate) use declarations::DeclarationRegistry;
pub(crate) use identity::{ROOT_MODULE_PATH, STANDALONE_PACKAGE_IDENTITY, standalone_module_path};
pub(crate) use token::RegistryToken;
