use bytemuck::NoUninit;
#[cfg(feature = "cpu_definition_export")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum MagicCowVec<'a, T> {
  Borrowed(&'a [T]),
  #[cfg(feature = "cpu_definition_export")]
  Owned(alloc::vec::Vec<T>),
}

#[cfg(feature = "cpu_definition_export")]
impl<'a, T: Serialize> Serialize for MagicCowVec<'a, T> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    match self {
      MagicCowVec::Borrowed(slice) => slice.serialize(serializer),
      MagicCowVec::Owned(vec) => vec.serialize(serializer),
    }
  }
}

#[cfg(feature = "cpu_definition_export")]
impl<'de: 'a, 'a, T: Deserialize<'de>> Deserialize<'de> for MagicCowVec<'a, T> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let vec = alloc::vec::Vec::<T>::deserialize(deserializer)?;
    Ok(MagicCowVec::Owned(vec))
  }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "cpu_definition_export", derive(Serialize, Deserialize))]
pub struct ShaderDefinition<'a> {
  pub name: &'a str,
  // TODO: add !rust_shader version!, description, author, shader version, etc.
  // TODO: Add keywords
  // TODO: bundle source code into the tar.gz not the shader definition
  #[cfg_attr(feature = "cpu_definition_export", serde(borrow))]
  pub parameters: MagicCowVec<'a, ShaderParameters<'a>>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "cpu_definition_export", derive(Serialize, Deserialize))]
pub enum ShaderParameters<'a> {
  #[cfg_attr(feature = "cpu_definition_export", serde(borrow))]
  ParameterIntSlider(ParameterIntSlider<'a>),
  // Add more parameter types as needed
}
pub trait ShaderParameter: Clone {
  type ParameterValue: NoUninit;
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "cpu_definition_export", derive(Serialize, Deserialize))]
pub struct ParameterIntSlider<'a> {
  min: i32,
  max: i32,
  default: i32,
  label: &'a str,
  description: &'a str,
  step: i32,
}

impl ShaderParameter for ParameterIntSlider<'_> {
  type ParameterValue = i32;
}
