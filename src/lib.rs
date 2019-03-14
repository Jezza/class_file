extern crate binform;
extern crate mutf8;

use std::convert::TryInto;
use std::marker::PhantomData;

use binform::*;
pub use binform::{
	ToBytes,
	FromBytes
};
pub use mutf8::{mstr, MString};

use crate::attr::Attribute;
use crate::ops::*;

pub mod ops;
pub mod attr;
pub mod macros;

const MAGIC: u32 = 0xCAFE_BABE;

def! {
	struct ClassFile('a) {
		#[binform(before(expect(ty = "u32", value = "MAGIC")))]
		minor_version: u16,
		major_version: u16,
		constant_pool: ConstantPool<'a>,
		access_flags: u16,
		this_class: CPIndex<'a, ClassInfo<'a>>,
		super_class: CPIndex<'a, ClassInfo<'a>>,
		#[binform(len = "u16")]
		interfaces: Vec<CPIndex<'a, ClassInfo<'a>>>,
		#[binform(len = "u16")]
		fields: Vec<FieldInfo<'a>>,
		#[binform(len = "u16")]
		methods: Vec<MethodInfo<'a>>,
		attributes: Attributes<'a>,		
	}
}

impl<'a> ClassFile<'a> {
	pub fn open<I: Read>(input: &mut I) -> ReadResult<ClassFile> {
		ClassFile::from_bytes(input)
	}
}

def! {
	struct ConstantPool('a) {
		#[binform(read = "read_constant_pool", write = "write_constant_pool")]
		entries: Vec<CPEntry<'a>>,
	}
}

fn read_constant_pool<'a, I: Read, BO: ByteOrder, L>(input: &mut I) -> ReadResult<Vec<CPEntry<'a>>> {
	let len = input.read_u16::<BO>()? - 1;
	let mut result = Vec::with_capacity(len as usize);
	for _ in 0..len {
		result.push(CPEntry::from_bytes(input)?);
	}
	Ok(result)
}

fn write_constant_pool<'a, O: Write, BO: ByteOrder, L>(value: &Vec<CPEntry<'a>>, output: &mut O) -> WriteResult {
	let len = value.len() + 1;
	if len > u16::max_value() as usize {
		return Err(WriteError::TooLarge(len));
	}
	output.write_u16::<BO>(len as u16)?;
	for e in value {
		e.to_bytes(output)?;
	}
	Ok(())
}

impl<'a> ConstantPool<'a> {
	pub fn index<T: 'a + CPType<'a>>(&'a self, index: CPIndex<'a, T>) -> Option<T::Output> {
		let entry = &self.entries[(index.index - 1) as usize];
		T::fetch(entry)
	}
}

impl<'a> IntoIterator for ConstantPool<'a> {
	type Item = CPEntry<'a>;
	type IntoIter = ::std::vec::IntoIter<CPEntry<'a>>;

	fn into_iter(self) -> Self::IntoIter {
		self.entries.into_iter()
	}
}

impl<'a, 'b> IntoIterator for &'b ConstantPool<'a> {
	type Item = &'b CPEntry<'a>;
	type IntoIter = ::std::slice::Iter<'b, CPEntry<'a>>;

	fn into_iter(self) -> Self::IntoIter {
		self.entries.iter()
	}
}

macro_rules! def_fetch {
	(
		$name:ident $( ( $($generics:tt)* ) )? => $into:ident
	) => {
		impl <'a, $( $($generics),* )?> CPType<'a> for $name $( < $($generics),* > )? {
			type Output = &'a Self;

			#[inline]
			fn fetch(entry: &'a CPEntry<'a>) -> Option<Self::Output> {
				if let CPEntry::$into(value) = entry {
					Some(value)
				} else {
					None
				}
			}
		}
	}
}

def_enum_of_structs! {
	#[binform(endian = "be", tag = "u8")]
	enum CPEntry('a) {
		#[binform(tag = "CONSTANT_CLASS_TAG")]
		@[binform(endian = "be")]
		Class(ClassInfo('a) {
			pub name_index: CPIndex<'a, UTF8Info<'a>>,
		}),
		#[binform(tag = "CONSTANT_FIELDREF_TAG")]
		@[binform(endian = "be")]
		FieldRef(FieldRefInfo('a) {
			pub class_index: CPIndex<'a, ClassInfo<'a>>,
			pub name_and_type_index: CPIndex<'a, NameAndTypeInfo<'a>>,
		}),
		#[binform(tag = "CONSTANT_METHODREF_TAG")]
		@[binform(endian = "be")]
		MethodRef(MethodRefInfo('a) {
			pub class_index: CPIndex<'a, ClassInfo<'a>>,
			pub name_and_type_index: CPIndex<'a, NameAndTypeInfo<'a>>,
		}),
		#[binform(tag = "CONSTANT_INTERFACE_METHODREF_TAG")]
		@[binform(endian = "be")]
		InterfaceMethodRef(InterfaceMethodRefInfo('a) {
			pub class_index: CPIndex<'a, ClassInfo<'a>>,
			pub name_and_type_index: CPIndex<'a, NameAndTypeInfo<'a>>
		}),
		#[binform(tag = "CONSTANT_STRING_TAG")]
		@[binform(endian = "be")]
		String(StringInfo('a) {
			pub string_index: CPIndex<'a, UTF8Info<'a>>
		}),
		#[binform(tag = "CONSTANT_INTEGER_TAG")]
		@[binform(endian = "be")]
		Integer(IntegerInfo {
			pub value: u32
		}),
		#[binform(tag = "CONSTANT_FLOAT_TAG")]
		@[binform(endian = "be")]
		Float(FloatInfo {
			pub value: u32
		}),
		#[binform(tag = "CONSTANT_LONG_TAG")]
		@[binform(endian = "be")]
		Long(LongInfo {
			pub high_bytes: u32,
			pub low_bytes: u32,
		}),
		#[binform(tag = "CONSTANT_DOUBLE_TAG")]
		@[binform(endian = "be")]
		Double(DoubleInfo {
			pub high_bytes: u32,
			pub low_bytes: u32,
		}),
		#[binform(tag = "CONSTANT_NAME_AND_TYPE_TAG")]
		@[binform(endian = "be")]
		NameAndType(NameAndTypeInfo('a) {
			pub name_index: CPIndex<'a, UTF8Info<'a>>,
			pub descriptor_index: CPIndex<'a, UTF8Info<'a>>
		}),
		#[binform(tag = "CONSTANT_UTF8_TAG")]
		@[binform(endian = "be")]
		UTF8(#use UTF8Info('a)),
		#[binform(tag = "CONSTANT_METHOD_HANDLE_TAG")]
		@[binform(endian = "be")]
		MethodHandle(#use MethodHandleInfo('a)),
		#[binform(tag = "CONSTANT_METHOD_TYPE_TAG")]
		@[binform(endian = "be")]
		MethodType(MethodTypeInfo('a) {
			pub descriptor_index: CPIndex<'a, UTF8Info<'a>>
		}),
		#[binform(tag = "CONSTANT_DYNAMIC_TAG")]
		@[binform(endian = "be")]
		Dynamic(DynamicInfo('a) {
			pub bootstrap_method_attr_index: u16,
			pub name_and_type_index: CPIndex<'a, NameAndTypeInfo<'a>>
		}),
		#[binform(tag = "CONSTANT_INVOKE_DYNAMIC_TAG")]
		@[binform(endian = "be")]
		InvokeDynamic(InvokeDynamicInfo('a) {
			pub bootstrap_method_attr_index: u16,
			pub name_and_type_index: CPIndex<'a, NameAndTypeInfo<'a>>
		}),
		#[binform(tag = "CONSTANT_MODULE_TAG")]
		@[binform(endian = "be")]
		Module(ModuleInfo('a) {
			pub name_index: CPIndex<'a, UTF8Info<'a>>
		}),
		#[binform(tag = "CONSTANT_PACKAGE_TAG")]
		@[binform(endian = "be")]
		Package(PackageInfo('a) {
			pub name_index: CPIndex<'a, UTF8Info<'a>>
		})
	}
}

impl CPEntry<'_> {
	pub fn tag(&self) -> u8 {
		match self {
			CPEntry::Class(_) => CONSTANT_CLASS_TAG,
			CPEntry::FieldRef(_) => CONSTANT_FIELDREF_TAG,
			CPEntry::MethodRef(_) => CONSTANT_METHODREF_TAG,
			CPEntry::InterfaceMethodRef(_) => CONSTANT_INTERFACE_METHODREF_TAG,
			CPEntry::String(_) => CONSTANT_STRING_TAG,
			CPEntry::Integer(_) => CONSTANT_INTEGER_TAG,
			CPEntry::Float(_) => CONSTANT_FLOAT_TAG,
			CPEntry::Long(_) => CONSTANT_LONG_TAG,
			CPEntry::Double(_) => CONSTANT_DOUBLE_TAG,
			CPEntry::NameAndType(_) => CONSTANT_NAME_AND_TYPE_TAG,
			CPEntry::UTF8(_) => CONSTANT_UTF8_TAG,
			CPEntry::MethodHandle(_) => CONSTANT_METHOD_HANDLE_TAG,
			CPEntry::MethodType(_) => CONSTANT_METHOD_TYPE_TAG,
			CPEntry::Dynamic(_) => CONSTANT_DYNAMIC_TAG,
			CPEntry::InvokeDynamic(_) => CONSTANT_INVOKE_DYNAMIC_TAG,
			CPEntry::Module(_) => CONSTANT_MODULE_TAG,
			CPEntry::Package(_) => CONSTANT_PACKAGE_TAG,
		}
	}
}

/// So, technically, I don't need the lifetime here, as we copy the string data,
/// but there's two reasons why I haven't removed it.
/// Firstly, this is an artefact from when this library initially used nom to parse all of the data,
/// which had the ability to perform a no-copy read.
///
/// Secondly, because I had already wrote all of the code that supports this stuff with lifetime of the input in mind,
/// it's a good idea to keep it around in the case/event/with the idea that I'll come back to it and finally add
/// no-copy support.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct UTF8Info<'a> {
	_marker: PhantomData<&'a ()>,
	pub data: MString
}
def_fetch!(UTF8Info('a) => UTF8);

impl<'a> ToBytes<BigEndian> for UTF8Info<'a> {
	fn to_bytes<O: Write>(&self, output: &mut O) -> WriteResult {
		let data = self.data.as_bytes();
		let len = data.len();
		let len = match len.try_into() {
			Result::Err(_e) => return Err(WriteError::TooLarge(len)),
			Result::Ok(len) => len,
		};
		output.write_u16::<BigEndian>(len)?;
		output.write_all(data)?;
		Ok(())
	}
}

impl<'a> FromBytes<BigEndian> for UTF8Info<'a> {
	type Output = Self;

	fn from_bytes<I: Read>(input: &mut I) -> ReadResult<Self::Output> {
		let len = input.read_u16::<BigEndian>()?;
		let mut data = Vec::with_capacity(len as usize);
		input.take(len as u64).read_to_end(&mut data)?;
		let data = unsafe { MString::from_mutf8_unchecked(data) };
		Ok(UTF8Info {
			_marker: PhantomData,
			data
		})
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, ToBytes, FromBytes)]
#[binform(endian = "be", tag = "u8")]
pub enum MethodHandleInfo<'a> {
//	#[binform(tag)]
//	FieldRef {
//	}
	#[binform(tag = "55")]
	Value(PhantomData<&'a ()>)
}
def_fetch!(MethodHandleInfo('a) => MethodHandle);

//cp_entry! {
//	enum MethodHandleInfo<'a> = (reference_kind: be_u8) {
//		FieldRef (1 | 2 | 3 | 4) {
//			reference_kind: u8 = (value!(reference_kind)),
//			reference_index: CPIndex<'a, FieldRefInfo<'a>> = (pt!(CPIndex)),
//		},
//		MethodRef (5 | 8 | 6 | 7) {
//			reference_kind: u8 = (value!(reference_kind)),
//			reference_index: CPIndex<'a, MethodRefInfo<'a>> = (pt!(CPIndex)),
//		},
//		InterfaceMethodRef (9) {
//			reference_kind: u8 = (value!(reference_kind)),
//			reference_index: CPIndex<'a, InterfaceMethodRefInfo<'a>> = (pt!(CPIndex)),
//		},
//	} => MethodHandle
//}

#[derive(Debug, Eq, PartialEq, Hash, Clone, ToBytes, FromBytes)]
#[binform(endian = "be")]
pub struct FieldInfo<'a> {
	access_flags: u16,
	name_index: CPIndex<'a, UTF8Info<'a>>,
	descriptor_index: CPIndex<'a, UTF8Info<'a>>,
	attributes: Attributes<'a>,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, ToBytes, FromBytes)]
#[binform(endian = "be")]
pub struct MethodInfo<'a> {
	access_flags: u16,
	name_index: CPIndex<'a, UTF8Info<'a>>,
	descriptor_index: CPIndex<'a, UTF8Info<'a>>,
	attributes: Attributes<'a>,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, ToBytes, FromBytes)]
#[binform(endian = "be")]
pub struct Attributes<'a> {
	#[binform(len = "u16")]
	attributes: Vec<AttributeInfo<'a>>
}

impl<'a> Attributes<'a> {
	pub fn iter(&self) -> impl Iterator<Item = &AttributeInfo<'a>> {
		self.attributes.iter()
	}

	pub fn named(&self, cp: &ConstantPool<'a>, name: &str) -> Option<&AttributeInfo<'a>> {
		for attr in &self.attributes {
			let info = cp.index(attr.attribute_name_index).expect("Unable to locate attribute_name_index in constant pool");
			if info.data.to_utf8() == name {
				return Some(attr);
			}
		}
		None
	}

	pub fn get<T: Attribute<'a>>(&self, cp: &ConstantPool<'a>) -> Option<T> {
		T::from_attributes(self, cp)
	}
}

impl<'a> IntoIterator for Attributes<'a> {
	type Item = AttributeInfo<'a>;
	type IntoIter = ::std::vec::IntoIter<AttributeInfo<'a>>;

	fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
		self.attributes.into_iter()
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, ToBytes, FromBytes)]
#[binform(endian = "be")]
pub struct AttributeInfo<'a> {
	attribute_name_index: CPIndex<'a, UTF8Info<'a>>,
	#[binform(read = "read_attr_info", write = "write_attr_info")]
	info: Vec<u8>,
}

fn read_attr_info<I: Read, BO: ByteOrder, L>(input: &mut I) -> ReadResult<Vec<u8>> {
	let len = input.read_u32::<BO>()?;
	let mut result = Vec::with_capacity(len as usize);
	input.take(len as u64)
		.read_to_end(&mut result)?;
	Ok(result)
}

fn write_attr_info<O: Write, BO: ByteOrder, L>(value: &Vec<u8>, output: &mut O) -> WriteResult {
	let len = value.len();
	if len > u32::max_value() as usize {
		return Err(WriteError::TooLarge(len));
	}
	output.write_u32::<BO>(len as u32)?;
	output.write_all(value)?;
	Ok(())
}

pub trait CPType<'a> {
	type Output;

	fn fetch(entry: &'a CPEntry<'a>) -> Option<Self::Output>;
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct CPIndex<'a, T: 'a + CPType<'a>> {
	pub index: u16,
	_marker: PhantomData<&'a T>,
}

impl<'a, T: 'a + CPType<'a>> Clone for CPIndex<'a, T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<'a, T: 'a + CPType<'a>> Copy for CPIndex<'a, T> {}

impl<'a, T: 'a + CPType<'a>, BO: ByteOrder, L> ToBytes<BO, L> for CPIndex<'a, T> {
	fn to_bytes<O: Write>(&self, output: &mut O) -> WriteResult {
		output.write_u16::<BO>(self.index)?;
		Ok(())
	}
}

impl<'a, T: 'a + CPType<'a>, BO: ByteOrder, L> FromBytes<BO, L> for CPIndex<'a, T> {
	type Output = Self;

	fn from_bytes<I: Read>(input: &mut I) -> ReadResult<Self::Output> {
		let value = input.read_u16::<BO>()?;
		let index = CPIndex {
			index: value,
			_marker: PhantomData,
		};
		Ok(index)
	}
}

impl<'a, T: 'a + CPType<'a>> CPIndex<'a, T> {
	pub fn read_non_zero<I: Read, BO: ByteOrder, L>(input: &mut I) -> ReadResult<Option<Self>> {
		let value = input.read_u16::<BO>()?;
		if value == 0 {
			return Ok(None);
		}
		Ok(Some(CPIndex {
			index: value,
			_marker: PhantomData,
		}))
	}

	pub fn write_non_zero<O: Write, BO: ByteOrder, L>(value: &Option<Self>, output: &mut O) -> WriteResult {
		match value {
			None => {
				output.write_u16::<BO>(0)?;
			}
			Some(value) => {
				output.write_u16::<BO>(value.index)?;
			}
		}
		Ok(())
	}
}
