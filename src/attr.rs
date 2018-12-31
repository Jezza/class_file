use nom::*;

use crate::*;

pub trait Attribute<'a>: Sized {
	fn from_attributes(attributes: &Attributes<'a>, cp: &ConstantPool<'a>) -> Option<Self>;
}

macro_rules! attr {
	(struct $type:ident <$first:lifetime $(,$rest:lifetime)*> {
		$($field_name:ident: $field_type:ty = ($($field_parser:tt)*)),* $(,)?
	}) => {
		parser! {
			struct $type<$first $(,$rest)*> {
				$($field_name: $field_type = ($($field_parser)*)),*
			}
		}

		impl<$first $(,$rest)*> Attribute<$first> for $type<$first $(,$rest)*> {
			fn from_attributes(attributes: &Attributes<$first>, cp: &ConstantPool<$first>) -> Option<Self> {
				let info = attributes.named(cp, stringify!($type))?;
				let result = $type::parse(info.info);
				if let Ok(value) = result {
					Some(value.1)
				} else {
					None
				}
			}
		}
	};
	(struct $type:ident {
		$($field_name:ident: $field_type:ty = ($($field_parser:tt)*)),* $(,)?
	}) => {
		parser! {
			struct $type {
				$($field_name: $field_type = ($($field_parser)*)),*
			}
		}

		impl<'a> Attribute<'a> for $type {
			fn from_attributes(attributes: &Attributes<'a>, cp: &ConstantPool<'a>) -> Option<Self> {
				let info = attributes.named(cp, stringify!($type))?;
		
				let result = $type::parse(info.info);
				if let Ok(value) = result {
					Some(value.1)
				} else {
					None
				}
			}
		}
	};
}

macro_rules! table {
	(struct $table:ident $(<$first:lifetime $(,$rest:lifetime)*>)? ($($table_parser:tt)*) => struct $type:ident {
		$($field_name:ident: $field_type:ty = ($($field_parser:tt)*)),*
	}) => {
		parser! {
			struct $type $(<$first $(,$rest)*>)? {
				$($field_name: $field_type = ($($field_parser)*)),*
			}
		}
		table!(struct $table $(<$first $(,$rest)*>)? ($($table_parser)*) => $type;);
	};
	(struct $table:ident $(<$first:lifetime $(,$rest:lifetime)*>)? ($($table_parser:tt)*) => $type:ident;) => {
		attr! {
			struct $table $(<$first $(,$rest)*>)? {
				data: Vec<$type $(<$first $(,$rest)*> )? > = (length_count!($($table_parser)*, pt!($type)))
			}
		}
	};
}

macro_rules! singleton {
	(struct $type:ident) => {
		#[derive(Debug, Eq, PartialEq, Hash, Clone)]
		pub struct $type;

		impl<'a> Attribute<'a> for $type {
			fn from_attributes(attributes: &Attributes<'a>, cp: &ConstantPool<'a>) -> Option<Self> {
				attributes.named(cp, stringify!($type))?;
				Some($type)
			}
		}
	};
}

attr! {
	struct ConstantValue<'a> {
		constantvalue_index: CPIndex<'a, ConstantValueInfo<'a>> = (pt!(CPIndex))
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum ConstantValueInfo<'a> {
	Integer(&'a IntegerInfo),
	Float(&'a FloatInfo),
	Long(&'a LongInfo),
	Double(&'a DoubleInfo),
	String(&'a StringInfo<'a>),
}

impl<'a> CPType<'a> for ConstantValueInfo<'a> {
	type Output = ConstantValueInfo<'a>;

	fn fetch(entry: &'a CPEntry<'a>) -> Option<Self::Output> {
		use crate::ops::{
			CONSTANT_INTEGER_TAG,
			CONSTANT_FLOAT_TAG,
			CONSTANT_LONG_TAG,
			CONSTANT_DOUBLE_TAG,
			CONSTANT_STRING_TAG,
		};
		match entry.tag() {
			CONSTANT_INTEGER_TAG => {
				if let CPEntry::Integer(ref info) = entry {
					Some(ConstantValueInfo::Integer(info))
				} else {
					None
				}
			}
			CONSTANT_FLOAT_TAG => {
				if let CPEntry::Float(ref info) = entry {
					Some(ConstantValueInfo::Float(info))
				} else {
					None
				}
			}
			CONSTANT_LONG_TAG => {
				if let CPEntry::Long(ref info) = entry {
					Some(ConstantValueInfo::Long(info))
				} else {
					None
				}
			}
			CONSTANT_DOUBLE_TAG => {
				if let CPEntry::Double(ref info) = entry {
					Some(ConstantValueInfo::Double(info))
				} else {
					None
				}
			}
			CONSTANT_STRING_TAG => {
				if let CPEntry::String(ref info) = entry {
					Some(ConstantValueInfo::String(info))
				} else {
					None
				}
			}
			_ => None,
		}
	}
}

attr! {
	struct Code<'a> {
		max_stack: u16 = (be_u16),
		max_locals: u16 = (be_u16),
		code: &'a [u8] = (length_data!(be_u32)),
		exception_table: Vec<Exception<'a>> = (length_count!(be_u16, pt!(Exception))),
		attributes: Attributes<'a> = (pt!(Attributes))
	}
}

parser! {
	struct Exception<'a> {
		start_pc: u16 = (be_u16),
		end_pc: u16 = (be_u16),
		handler_pc: u16 = (be_u16),
		catch_type: CPIndex<'a, ClassInfo<'a>> = (pt!(CPIndex))
	}
}

table! {
	struct StackMapTable<'a>(be_u16) => StackMapFrame;
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum StackMapFrame<'a> {
	SameFrame,
	SameLocals(VerificationTypeInfo<'a>),
	SameLocalsExtended {
		offset: u16,
		verification_type_info: VerificationTypeInfo<'a>,
	},
	ChopFrame(u16),
	SameFrameExtended(u16),
	AppendFrame {
		offset_delta: u16,
		locals: Vec<VerificationTypeInfo<'a>>,
	},
	FullFrame {
		offset_delta: u16,
		locals: Vec<VerificationTypeInfo<'a>>,
		stack: Vec<VerificationTypeInfo<'a>>,
	}
}

// @TODO Jezza - 31 Dec. 2018: Swap out these "magic" numbers with constants... 
impl<'a> StackMapFrame<'a> {
	named!(pub parse<StackMapFrame>, switch!(be_u8,
		0..=63 => value!(StackMapFrame::SameFrame)
		|
		64..=127 => do_parse!(
			verification_type_info: pt!(VerificationTypeInfo) >>
			(StackMapFrame::SameLocals(verification_type_info))
		)
		|
		247 => do_parse!(
			offset: be_u16 >>
			verification_type_info: pt!(VerificationTypeInfo) >>
			(StackMapFrame::SameLocalsExtended {
				offset,
				verification_type_info
			})
		)
		|
		148..=250 => do_parse!(
			offset_delta: be_u16 >>
			(StackMapFrame::ChopFrame(offset_delta))
		)
		|
		251 => do_parse!(
			offset: be_u16 >>
			(StackMapFrame::SameFrameExtended(offset))
		)
		|
		252 => do_parse!(
			offset_delta: be_u16 >>
			locals: length_count!(value!(1), pt!(VerificationTypeInfo)) >>
			(StackMapFrame::AppendFrame {
				offset_delta,
				locals,
			})
		)
		|
		253 => do_parse!(
			offset_delta: be_u16 >>
			locals: length_count!(value!(2), pt!(VerificationTypeInfo)) >>
			(StackMapFrame::AppendFrame {
				offset_delta,
				locals,
			})
		)
		|
		254 => do_parse!(
			offset_delta: be_u16 >>
			locals: length_count!(value!(3), pt!(VerificationTypeInfo)) >>
			(StackMapFrame::AppendFrame {
				offset_delta,
				locals,
			})
		)
		|
		255 => do_parse!(
			offset_delta: be_u16 >>
			locals: length_count!(be_u16, pt!(VerificationTypeInfo)) >>
			stack: length_count!(be_u16, pt!(VerificationTypeInfo)) >>
			(StackMapFrame::FullFrame {
				offset_delta,
				locals,
				stack,
			})
		)
	));
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum VerificationTypeInfo<'a> {
	Top,
	Integer,
	Float,
	Double,
	Long,
	Null,
	UninitializedThis,
	ObjectVariable(CPIndex<'a, ClassInfo<'a>>),
	Uninitialized(u16),
}

const ITEM_TOP: u8 = 0;
const ITEM_INTEGER: u8 = 1;
const ITEM_FLOAT: u8 = 2;
const ITEM_DOUBLE: u8 = 3;
const ITEM_LONG: u8 = 4;
const ITEM_NULL: u8 = 5;
const ITEM_UNINITIALIZED_THIS: u8 = 6;
const ITEM_OBJECT: u8 = 7;
const ITEM_UNINITIALIZED: u8 = 8;

impl<'a> VerificationTypeInfo<'a> {
	named!(pub parse<VerificationTypeInfo>, switch!(be_u8,
		ITEM_TOP => value!(VerificationTypeInfo::Top)
		|
		ITEM_INTEGER => value!(VerificationTypeInfo::Integer)
		|
		ITEM_FLOAT => value!(VerificationTypeInfo::Float)
		|
		ITEM_DOUBLE => value!(VerificationTypeInfo::Double)
		|
		ITEM_LONG => value!(VerificationTypeInfo::Long)
		|
		ITEM_NULL => value!(VerificationTypeInfo::Null)
		|
		ITEM_UNINITIALIZED_THIS => value!(VerificationTypeInfo::UninitializedThis)
		|
		ITEM_OBJECT => do_parse!(
			class_index: pt!(CPIndex) >>
			(VerificationTypeInfo::ObjectVariable(class_index))
		)
		|
		ITEM_UNINITIALIZED => do_parse!(
			offset: be_u16 >>
			(VerificationTypeInfo::Uninitialized(offset))
		)
	));
}

attr! {
	struct Exceptions<'a> {
		table: Vec<CPIndex<'a, ClassInfo<'a>>> = (length_count!(be_u16, pt!(CPIndex)))
	}
}

table! {
	struct InnerClasses<'a>(be_u16) => struct InnerClass {
		inner_class_info_index: CPIndex<'a, ClassInfo<'a>> = (pt!(CPIndex)),
		outer_class_info_index: Option<CPIndex<'a, ClassInfo<'a>>> = (call!(CPIndex::parse_non_zero)),
		inner_name_index: Option<CPIndex<'a, UTF8Info<'a>>> = (call!(CPIndex::parse_non_zero)),
		inner_class_access_flags: u16 = (be_u16)
	}
}

attr! {
	struct EnclosingMethod<'a> {
		class_index: CPIndex<'a, ClassInfo<'a>> = (pt!(CPIndex)),
		method_index: CPIndex<'a, NameAndTypeInfo<'a>> = (pt!(CPIndex))
	}
}

singleton!(struct Synthetic);

attr! {
	struct Signature<'a> {
		class_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
	}
}

attr! {
	struct SourceFile<'a> {
		sourcefile_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct SourceDebugExtension<'a> {
	pub data: &'a mstr
}

impl<'a> Attribute<'a> for SourceDebugExtension<'a> {
	fn from_attributes(attributes: &Attributes<'a>, cp: &ConstantPool<'a>) -> Option<Self> {
		let info = attributes.named(cp, "SourceDebugExtension")?;
		let data = mstr::from_mutf8_unchecked(info.info);
		Some(SourceDebugExtension {
			data
		})
	}
}

table! {
	struct LineNumberTable(be_u16) => struct LineNumber {
		start_pc: u16 = (be_u16),
		line_number: u16 = (be_u16)
	}
}

table! {
	struct LocalVariableTable<'a>(be_u16) => struct LocalVariable {
		start_pc: u16 = (be_u16),
		length: u16 = (be_u16),
		name_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		descriptor_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		index: u16 = (be_u16)
	}
}

table! {
	struct LocalVariableTypeTable<'a>(be_u16) => struct LocalVariableType {
		start_pc: u16 = (be_u16),
		length: u16 = (be_u16),
		name_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		signature_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		index: u16 = (be_u16)
	}
}

singleton!(struct Deprecated);

table! {
	struct RuntimeVisibleAnnotations<'a>(be_u16) => Annotation;
}

table! {
	struct RuntimeInvisibleAnnotations<'a>(be_u16) => Annotation;
}

parser! {
	struct Annotation<'a> {
		type_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		element_value_pairs: Vec<ElementValuePair<'a>> = (length_count!(be_u16, pt!(ElementValuePair)))
	}
}

parser! {
	struct ElementValuePair<'a> {
		element_name_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		element_value: ElementValue<'a> = (pt!(ElementValue))
	}
}

parser! {
	enum ElementValue<'a> = (kind: be_u8) {
		Byte(b'B') {
			const_value_index: CPIndex<'a, IntegerInfo> = (pt!(CPIndex))
		},
		Char(b'C') {
			const_value_index: CPIndex<'a, IntegerInfo> = (pt!(CPIndex))
		},
		Double(b'D') {
			const_value_index: CPIndex<'a, DoubleInfo> = (pt!(CPIndex))
		},
		Float(b'F') {
			const_value_index: CPIndex<'a, FloatInfo> = (pt!(CPIndex))
		},
		Integer(b'I') {
			const_value_index: CPIndex<'a, IntegerInfo> = (pt!(CPIndex))
		},
		Long(b'J') {
			const_value_index: CPIndex<'a, LongInfo> = (pt!(CPIndex))
		},
		Short(b'S') {
			const_value_index: CPIndex<'a, IntegerInfo> = (pt!(CPIndex))
		},
		Boolean(b'Z') {
			const_value_index: CPIndex<'a, IntegerInfo> = (pt!(CPIndex))
		},
		String(b's') {
			const_value_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
		},
		Enum(b'e') {
			type_name_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
			const_name_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
		},
		Class(b'c') {
			class_info_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
		},
		Annotation(b'@') {
			annotation_value: Annotation<'a> = (pt!(Annotation))
		},
		Array(b'[') {
			array_value: Vec<ElementValue<'a>> = (length_count!(be_u16, pt!(ElementValue)))
		},
	}
}

table! {
	struct RuntimeVisibleParameterAnnotations<'a>(be_u8) => ParameterAnnotations;
}

table! {
	struct RuntimeInvisibleParameterAnnotations<'a>(be_u8) => ParameterAnnotations;
}

parser! {
	struct ParameterAnnotations<'a> {
		annotations: Vec<Annotation<'a>> = (length_count!(be_u16, pt!(Annotation)))
	}
}

table! {
	struct RuntimeVisibleTypeAnnotations<'a>(be_u16) => TypeAnnotation;
}

table! {
	struct RuntimeInvisibleTypeAnnotations<'a>(be_u16) => TypeAnnotation;
}

parser! {
	struct TypeAnnotation<'a> {
		target_info         : TargetInfo                = (pt!(TargetInfo)),
		target_path         : TypePath                  = (pt!(TypePath)),
		type_index          : CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
		element_value_pairs : Vec<ElementValuePair<'a>> = (length_count!(be_u16, pt!(ElementValuePair)))
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum TargetInfo {
	TypeParameter(u8),
	SuperType(u16),
	TypeParameterBound {
		type_parameter_index: u8,
		bound_index: u8,
	},
	Empty,
	FormalParameter(u8),
	Throws(u16),
	LocalVar {
		start_pc: u16,
		length: u16,
		index: u16,
	},
	Catch(u16),
	Offset(u16),
	TypeArgument {
		offset: u16,
		type_argument_index: u8
	},
}

impl TargetInfo {
	named!(pub parse<TargetInfo>, switch!(be_u8,
		0x00 | 0x01 => do_parse!(
			index: be_u8 >>
			(TargetInfo::TypeParameter(index))
		)
		|
		0x10 => do_parse!(
			index: be_u16 >>
			(TargetInfo::SuperType(index))
		)
		|
		0x11 | 0x12 => do_parse!(
			type_parameter_index: be_u8 >>
			bound_index: be_u8 >>
			(TargetInfo::TypeParameterBound {
				type_parameter_index,
				bound_index,
			})
		)
		|
		0x13 | 0x14 | 0x15 => value!(TargetInfo::Empty)
		|
		0x16 => do_parse!(
			index: be_u8 >>
			(TargetInfo::FormalParameter(index))
		)
		|
		0x17 => do_parse!(
			throws_type_index: be_u16 >>
			(TargetInfo::Throws(throws_type_index))
		)
		|
		0x40 | 0x41 => do_parse!(
			start_pc: be_u16 >>
			length: be_u16 >>
			index: be_u16 >>
			(TargetInfo::LocalVar {
				start_pc,
				length,
				index,
			})
		)
		|
		0x42 => do_parse!(
			exception_table_index: be_u16 >>
			(TargetInfo::Catch(exception_table_index))
		)
		|
		0x43 | 0x44 | 0x45 | 0x46 => do_parse!(
			offset: be_u16 >>
			(TargetInfo::Offset(offset))
		)
		|
		0x47 | 0x48 | 0x49 | 0x4A | 0x4B => do_parse!(
			offset: be_u16 >>
			type_argument_index: be_u8 >>
			(TargetInfo::TypeArgument {
				offset,
				type_argument_index,
			})
		)
	));
}

parser! {
	struct TypePath {
		data: Vec<TypePathSegment> = (length_count!(be_u8, pt!(TypePathSegment)))
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum TypePathSegment {
	Array,
	NestedType,
	WildcardBound,
	TypeArgument(u8),
}

impl TypePathSegment {
	named!(pub parse<TypePathSegment>, do_parse!(
		type_path_kind: be_u8 >>
		type_argument_index: be_u8 >>
		segment: switch!(value!(type_path_kind),
			0 => value!(TypePathSegment::Array)
			|
			1 => value!(TypePathSegment::NestedType)
			|
			3 => value!(TypePathSegment::WildcardBound)
			|
			4 => value!(TypePathSegment::TypeArgument(type_argument_index))
		) >>
		(segment)
	));
}

attr! {
	struct AnnotationDefault<'a> {
		default_value: ElementValue<'a> = (pt!(ElementValue))
	}
}

table! {
	struct BootstrapMethods<'a>(be_u16) => struct BootstrapMethod {
		bootstrap_method_ref: CPIndex<'a, MethodHandleInfo<'a>> = (pt!(CPIndex)),
		bootstrap_arguments: Vec<CPIndex<'a, LoadableConstant<'a>>> = (length_count!(be_u16, pt!(CPIndex)))
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum LoadableConstant<'a> {
	Integer(&'a IntegerInfo),
	Float(&'a FloatInfo),
	Long(&'a LongInfo),
	Double(&'a DoubleInfo),
	Class(&'a ClassInfo<'a>),
	String(&'a StringInfo<'a>),
	MethodHandle(&'a MethodHandleInfo<'a>),
	MethodType(&'a MethodTypeInfo<'a>),
	Dynamic(&'a DynamicInfo<'a>),
}

impl<'a> CPType<'a> for LoadableConstant<'a> {
	type Output = Self;

	fn fetch(entry: &'a CPEntry<'a>) -> Option<Self::Output> {
		use crate::ops::{
			CONSTANT_INTEGER_TAG,
			CONSTANT_FLOAT_TAG,
			CONSTANT_LONG_TAG,
			CONSTANT_DOUBLE_TAG,
			CONSTANT_STRING_TAG,
			CONSTANT_METHOD_HANDLE_TAG,
			CONSTANT_METHOD_TYPE_TAG,
			CONSTANT_DYNAMIC_TAG,
		};
		match entry.tag() {
			CONSTANT_INTEGER_TAG => {
				if let CPEntry::Integer(ref info) = entry {
					Some(LoadableConstant::Integer(info))
				} else {
					None
				}
			}
			CONSTANT_FLOAT_TAG => {
				if let CPEntry::Float(ref info) = entry {
					Some(LoadableConstant::Float(info))
				} else {
					None
				}
			}
			CONSTANT_LONG_TAG => {
				if let CPEntry::Long(ref info) = entry {
					Some(LoadableConstant::Long(info))
				} else {
					None
				}
			}
			CONSTANT_DOUBLE_TAG => {
				if let CPEntry::Double(ref info) = entry {
					Some(LoadableConstant::Double(info))
				} else {
					None
				}
			}
			CONSTANT_STRING_TAG => {
				if let CPEntry::String(ref info) = entry {
					Some(LoadableConstant::String(info))
				} else {
					None
				}
			}
			CONSTANT_METHOD_HANDLE_TAG => {
				if let CPEntry::MethodHandle(ref info) = entry {
					Some(LoadableConstant::MethodHandle(info))
				} else {
					None
				}
			}
			CONSTANT_METHOD_TYPE_TAG => {
				if let CPEntry::MethodType(ref info) = entry {
					Some(LoadableConstant::MethodType(info))
				} else {
					None
				}
			}
			CONSTANT_DYNAMIC_TAG => {
				if let CPEntry::Dynamic(ref info) = entry {
					Some(LoadableConstant::Dynamic(info))
				} else {
					None
				}
			}
			_ => None,
		}
	}
}

table! {
	struct MethodParameters<'a>(be_u16) => MethodParameter;
}

parser! {
	struct MethodParameter<'a> {
		name_index: Option<CPIndex<'a, UTF8Info<'a>>> = (call!(CPIndex::parse_non_zero)),
		access_flags: u16 = (be_u16)
	}
}

attr! {
	struct Module<'a> {
		module_name_index    : CPIndex<'a, ModuleInfo<'a>>         = (pt!(CPIndex)),
		module_flags         : u16                                 = (be_u16),
		module_version_index : Option<CPIndex<'a, ModuleInfo<'a>>> = (call!(CPIndex::parse_non_zero)),
		requires             : Vec<Requires<'a>>                   = (length_count!(be_u16, pt!(Requires))),
		exports              : Vec<Exports<'a>>                    = (length_count!(be_u16, pt!(Exports))),
		opens                : Vec<Opens<'a>>                      = (length_count!(be_u16, pt!(Opens))),
		uses                 : Vec<CPIndex<'a, ClassInfo<'a>>>     = (length_count!(be_u16, pt!(CPIndex))),
		provides             : Vec<Provides<'a>>                   = (length_count!(be_u16, pt!(Provides)))
	}
}

parser! {
	struct Requires<'a> {
		requires_index: CPIndex<'a, ModuleInfo<'a>> = (pt!(CPIndex)),
		requires_flags: u16 = (be_u16),
		requires_version_index: Option<CPIndex<'a, UTF8Info<'a>>> = (call!(CPIndex::parse_non_zero)),
	}
}

parser! {
	struct Exports<'a> {
		exports_index: CPIndex<'a, PackageInfo<'a>> = (pt!(CPIndex)),
		exports_flags: u16 = (be_u16),
		exports_to: Vec<CPIndex<'a, ModuleInfo<'a>>> = (length_count!(be_u16, pt!(CPIndex)))
	}
}

parser! {
	struct Opens<'a> {
		opens_index: CPIndex<'a, PackageInfo<'a>> = (pt!(CPIndex)),
		opens_flags: u16 = (be_u16),
		opens_to: Vec<CPIndex<'a, ModuleInfo<'a>>> = (length_count!(be_u16, pt!(CPIndex)))
	}
}

parser! {
	struct Provides<'a> {
		provides: CPIndex<'a, ClassInfo<'a>> = (pt!(CPIndex)),
		provides_with: Vec<CPIndex<'a, ClassInfo<'a>>> = (length_count!(be_u16, pt!(CPIndex)))
	}
}

attr! {
	struct ModulePackages<'a> {
		packages: Vec<CPIndex<'a, PackageInfo<'a>>> = (length_count!(be_u16, pt!(CPIndex)))
	}
}

attr! {
	struct ModuleMainClass<'a> {
		main_class_index: CPIndex<'a, ClassInfo<'a>> = (pt!(CPIndex))
	}
}

attr! {
	struct NestHost<'a> {
		host_class_index: CPIndex<'a, ClassInfo<'a>> = (pt!(CPIndex))
	}
}

attr! {
	struct NestMembers<'a> {
		classes: Vec<CPIndex<'a, ClassInfo<'a>>> = (length_count!(be_u16, pt!(CPIndex)))
	}
}
