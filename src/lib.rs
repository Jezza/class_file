#[macro_use]
extern crate nom;

extern crate mutf8;

#[macro_use]
mod parsing;

pub mod ops;
pub mod attr;

use nom::*;
pub use mutf8::{MString, mstr};
pub use nom::Err as Err;

use crate::ops::*;
use crate::attr::Attribute;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ClassFile<'a> {
	pub minor_version: u16,
	pub major_version: u16,
	pub constant_pool: ConstantPool<'a>,
	pub access_flags: u16,
	pub this_class: CPIndex<'a, ClassInfo<'a>>,
	pub super_class: CPIndex<'a, ClassInfo<'a>>,
	pub interfaces: Vec<CPIndex<'a, ClassInfo<'a>>>,
	pub fields: Vec<FieldInfo<'a>>,
	pub methods: Vec<MethodInfo<'a>>,
	pub attributes: Attributes<'a>,
}

impl<'a> ClassFile<'a> {
	named!(pub parse<ClassFile>, do_parse!(
		tag!([0xCA, 0xFE, 0xBA, 0xBE]) >>
		minor_version: be_u16 >>
		major_version: be_u16 >>
		constant_pool: pt!(ConstantPool) >>
		access_flags: be_u16 >>
		this_class: pt!(CPIndex) >>
		super_class: pt!(CPIndex) >>
		interfaces: length_count!(be_u16, pt!(CPIndex)) >>
		fields: length_count!(be_u16, pt!(FieldInfo)) >>
		methods: length_count!(be_u16, pt!(MethodInfo)) >>
		attributes: pt!(Attributes) >>
		(ClassFile {
			minor_version,
			major_version,
			constant_pool,
			access_flags,
			this_class,
			super_class,
			interfaces,
			fields,
			methods,
			attributes,
		})
	));
}


#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ConstantPool<'a> {
	pub entries: Vec<CPEntry<'a>>,
}

impl<'a> ConstantPool<'a> {
	named!(pub parse<ConstantPool>, do_parse!(
		entries: length_count!(do_parse!(
			count: be_u16 >>
			(count - 1)
		), pt!(CPEntry)) >>
		(ConstantPool {
			entries,
		})
	));

	pub fn index<T: 'a + CPType<'a>>(&'a self, index: CPIndex<'a, T>) -> Option<T::Output> {
		let entry = &self.entries[(index.index - 1) as usize];
		T::fetch(entry)
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum CPEntry<'a> {
	Class(ClassInfo<'a>),
	FieldRef(FieldRefInfo<'a>),
	MethodRef(MethodRefInfo<'a>),
	InterfaceMethodRef(InterfaceMethodRefInfo<'a>),
	String(StringInfo<'a>),
	Integer(IntegerInfo),
	Float(FloatInfo),
	Long(LongInfo),
	Double(DoubleInfo),
	NameAndType(NameAndTypeInfo<'a>),
	UTF8(UTF8Info<'a>),
	MethodHandle(MethodHandleInfo<'a>),
	MethodType(MethodTypeInfo<'a>),
	Dynamic(DynamicInfo<'a>),
	InvokeDynamic(InvokeDynamicInfo<'a>),
	Module(ModuleInfo<'a>),
	Package(PackageInfo<'a>),
}

macro_rules! def_fetch {
    ($type:ident <$first:lifetime $(,$rest:lifetime)*> => $deconstruct:ident) => {
		impl <$first $(,$rest)*> CPType<$first> for $type <$first $(,$rest)*> {
			type Output = & $first Self;
			
			#[inline]
			fn fetch(entry: & $first CPEntry< $first >) -> Option<Self::Output> {
				if let CPEntry::$deconstruct(info) = entry {
					Some(info)
				} else {
					None
				}
			}
		}
    };
    ($type:ident => $deconstruct:ident) => {
		impl <'a> CPType<'a> for $type {
			type Output = &'a Self;
			
			#[inline]
			fn fetch(entry: &'a CPEntry<'a>) -> Option<Self::Output> {
				if let CPEntry::$deconstruct(info) = entry {
					Some(info)
				} else {
					None
				}
			}
		}
    };
}

macro_rules! cp_entry {
	(struct $type:ident $(<$first:lifetime $(,$rest:lifetime)*>)? {
		$($field_name:ident: $field_type:ty = ($($field_parser:tt)*)),* $(,)?
	} => $deconstruct:ident) => {
		parser! {
			struct $type $(<$first $(,$rest)*>)? {
				$($field_name: $field_type = ($($field_parser)*)),*
			}
		}
		def_fetch!($type $(<$first $(,$rest)*>)? => $deconstruct);
    };
    (enum $type:ident $(<$first:lifetime $(,$rest:lifetime)*>)? = ($ident:ident: $($tag_parser:tt)*) {
		$(
			$variant:ident ($($variant_tag:tt)*) {
				$($field_name:ident: $field_type:ty = ($($field_parser:tt)*)),* $(,)?
			}
		),* $(,)?
	} => $deconstruct:ident) => {
		parser! {
			enum $type $(<$first $(,$rest)*>)? = ($ident: $($tag_parser)*) {
				$(
					$variant ($($variant_tag)*) {
						$($field_name: $field_type = ($($field_parser)*)),*
					}
				),*
			}
		}

		def_fetch!($type $(<$first $(,$rest)*>)? => $deconstruct);
	}
}

cp_entry! {
	struct ClassInfo<'a> {
		name_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
	} => Class
}

cp_entry! {
	struct FieldRefInfo<'a> {
		class_index: CPIndex<'a, ClassInfo<'a>> = (pt!(CPIndex)),
		name_and_type_index: CPIndex<'a, NameAndTypeInfo<'a>> = (pt!(CPIndex))
	} => FieldRef
}

cp_entry! {
	struct MethodRefInfo<'a> {
		class_index: CPIndex<'a, ClassInfo<'a>> = (pt!(CPIndex)),
		name_and_type_index: CPIndex<'a, NameAndTypeInfo<'a>> = (pt!(CPIndex))
	} => MethodRef
}

cp_entry! {
	struct InterfaceMethodRefInfo<'a> {
		class_index: CPIndex<'a, ClassInfo<'a>> = (pt!(CPIndex)),
		name_and_type_index: CPIndex<'a, NameAndTypeInfo<'a>> = (pt!(CPIndex))
	} => InterfaceMethodRef
}

cp_entry! {
	struct StringInfo<'a> {
		string_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
	} => String
}

cp_entry! {
	struct IntegerInfo {
		bytes: u32 = (be_u32),
	} => Integer
}

cp_entry! {
	struct FloatInfo {
		bytes: u32 = (be_u32),
	} => Float
}

cp_entry! {
	struct LongInfo {
		high_bytes: u32 = (be_u32),
		low_bytes: u32 = (be_u32),
	} => Long
}

cp_entry! {
	struct DoubleInfo {
		high_bytes: u32 = (be_u32),
		low_bytes: u32 = (be_u32),
	} => Double
}

cp_entry! {
	struct NameAndTypeInfo<'a> {
		name_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		descriptor_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
	} => NameAndType
}

cp_entry! {
	struct UTF8Info<'a> {
		data: &'a mstr = (do_parse!(
			bytes: length_data!(be_u16) >>
			(mstr::from_mutf8_unchecked(bytes))
		)),
	} => UTF8
}

cp_entry! {
	enum MethodHandleInfo<'a> = (reference_kind: be_u8) {
		FieldRef (1 | 2 | 3 | 4) {
			reference_kind: u8 = (value!(reference_kind)),
			reference_index: CPIndex<'a, FieldRefInfo<'a>> = (pt!(CPIndex)),
		},
		MethodRef (5 | 8 | 6 | 7) {
			reference_kind: u8 = (value!(reference_kind)),
			reference_index: CPIndex<'a, MethodRefInfo<'a>> = (pt!(CPIndex)),
		},
		InterfaceMethodRef (9) {
			reference_kind: u8 = (value!(reference_kind)),
			reference_index: CPIndex<'a, InterfaceMethodRefInfo<'a>> = (pt!(CPIndex)),
		},
	} => MethodHandle
}

cp_entry! {
	struct MethodTypeInfo<'a> {
		descriptor_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
	} => MethodType
}

cp_entry! {
	struct DynamicInfo<'a> {
		bootstrap_method_attr_index: u16 = (be_u16),
		name_and_type_index: CPIndex<'a, NameAndTypeInfo<'a>> = (pt!(CPIndex))
	} => Dynamic
}

cp_entry! {
	struct InvokeDynamicInfo<'a> {
		bootstrap_method_attr_index: u16 = (be_u16),
		name_and_type_index: CPIndex<'a, NameAndTypeInfo<'a>> = (pt!(CPIndex))
	} => InvokeDynamic
}

cp_entry! {
	struct ModuleInfo<'a> {
		name_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
	} => Module
}

cp_entry! {
	struct PackageInfo<'a> {
		name_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
	} => Package
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

	named!(pub parse<CPEntry>, switch!(be_u8,
		CONSTANT_CLASS_TAG => do_parse!(
			info: pt!(ClassInfo) >>
			(CPEntry::Class(info))
		)
		|
		CONSTANT_FIELDREF_TAG => do_parse!(
			info: pt!(FieldRefInfo) >>
			(CPEntry::FieldRef(info))
		)
		|
		CONSTANT_METHODREF_TAG => do_parse!(
			info: pt!(MethodRefInfo) >>
			(CPEntry::MethodRef(info))
		)
		|
		CONSTANT_INTERFACE_METHODREF_TAG => do_parse!(
			info: pt!(InterfaceMethodRefInfo) >>
			(CPEntry::InterfaceMethodRef(info))
		)
		|
		CONSTANT_STRING_TAG => do_parse!(
			info: pt!(StringInfo) >>
			(CPEntry::String(info))
		)
		|
		CONSTANT_INTEGER_TAG => do_parse!(
			info: pt!(IntegerInfo) >>
			(CPEntry::Integer(info))
		)
		|
		CONSTANT_FLOAT_TAG => do_parse!(
			info: pt!(FloatInfo) >>
			(CPEntry::Float(info))
		)
		|
		CONSTANT_LONG_TAG => do_parse!(
			info: pt!(LongInfo) >>
			(CPEntry::Long(info))
		)
		|
		CONSTANT_DOUBLE_TAG => do_parse!(
			info: pt!(DoubleInfo) >>
			(CPEntry::Double(info))
		)
		|
		CONSTANT_NAME_AND_TYPE_TAG => do_parse!(
			info: pt!(NameAndTypeInfo) >>
			(CPEntry::NameAndType(info))
		)
		|
		CONSTANT_UTF8_TAG => do_parse!(
			info: pt!(UTF8Info) >>
			(CPEntry::UTF8(info))
		)
		|
		CONSTANT_METHOD_HANDLE_TAG => do_parse!(
			info: pt!(MethodHandleInfo) >>
			(CPEntry::MethodHandle(info))
		)
		|
		CONSTANT_METHOD_TYPE_TAG => do_parse!(
			info: pt!(MethodTypeInfo) >>
			(CPEntry::MethodType(info))
		)
		|
		CONSTANT_DYNAMIC_TAG => do_parse!(
			info: pt!(DynamicInfo) >>
			(CPEntry::Dynamic(info))
		)
		|
		CONSTANT_INVOKE_DYNAMIC_TAG => do_parse!(
			info: pt!(InvokeDynamicInfo) >>
			(CPEntry::InvokeDynamic(info))
		)
		|
		CONSTANT_MODULE_TAG => do_parse!(
			info: pt!(ModuleInfo) >>
			(CPEntry::Module(info))
		)
		|
		CONSTANT_PACKAGE_TAG => do_parse!(
			info: pt!(PackageInfo) >>
			(CPEntry::Package(info))
		)
	));
}

parser! {
	struct FieldInfo<'a> {
		access_flags     : u16                       = (be_u16),
		name_index       : CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		descriptor_index : CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		attributes       : Attributes<'a>            = (pt!(Attributes))
	}
}

parser! {
	struct MethodInfo<'a> {
		access_flags     : u16                       = (be_u16),
		name_index       : CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		descriptor_index : CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		attributes       : Attributes<'a>            = (pt!(Attributes))
	}
}

parser! {
	struct Attributes<'a> {
		attributes : Vec<AttributeInfo<'a>> = (length_count!(be_u16, pt!(AttributeInfo)))
	}
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

parser! {
	struct AttributeInfo<'a> {
		attribute_name_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		info: &'a [u8] = (length_data!(be_u32))
	}
}

pub trait CPType<'a> {
	type Output;

	fn fetch(entry: &'a CPEntry<'a>) -> Option<Self::Output>;
}

use std::marker::PhantomData;

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

impl<'a, T: 'a + CPType<'a>> Copy for CPIndex<'a, T> {
}

impl<'a, T: 'a + CPType<'a>> CPIndex<'a, T> {
	named!(pub parse<CPIndex<'a, T>>, do_parse!(
		index: be_u16 >>
		(CPIndex {
			index,
			_marker: PhantomData,
		})
	));

	named!(pub parse_non_zero<Option<CPIndex<'a, T>>>, do_parse!(
		index: be_u16 >>
		(
			if index == 0 {
				None
			} else {
				Some(CPIndex {
					index,
					_marker: PhantomData,
				})
			}
		)
	));
}
