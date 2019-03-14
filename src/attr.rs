use crate::*;
use std::io::Cursor;

pub trait Attribute<'a>: Sized {
	fn from_attributes(attributes: &Attributes<'a>, cp: &ConstantPool<'a>) -> Option<Self>;
}

macro_rules! impl_attr {
	($type:ident $( ( $($generics:tt),* ) )?) => {
		impl<'a $(, $($generics),* )?> Attribute<'a> for $type $( < $( $generics ),* > )? {
			fn from_attributes(attributes: &Attributes, cp: &ConstantPool) -> Option<Self> {
				let info = attributes.named(cp, stringify!($type))?;
				let data = &info.info;
		
				let mut input = Cursor::new(data);
				if let Ok(value) = <$type as FromBytes<BigEndian>>::from_bytes(&mut input) {
					return Some(value);
				}
				return None;
			}
		}
	}
}

macro_rules! attr {
	(
		struct $type:ident $( ( $($generics:tt),* ) )? {
			$($body:tt)*
		}
	) => {
		def! {
			struct $type $( ( $($generics),* ) )? {
				$($body)*
			}
		}
		impl_attr!($type $( ( $($generics),* ) )?);
	};
}

macro_rules! table {
	(
		@len = $len:literal;
		struct $table:ident $( ( $($generics:tt)* ) )? => $inner:ident;
	) => {
		def! {
			struct $table $( ( $($generics)* ) )? {
				#[binform(len = $len)]
				table: Vec<$inner $( < $($generics)* > )?>,
			}
		}
	};
	(
		@len = $len:literal;
		struct $table:ident $( ( $($generics:tt)* ) )? => struct $inner:ident {
			$($body:tt)*
		}
	) => {
		table! {
			@len = $len;
			struct $table $( ( $($generics)* ) )? => $inner;
		}

		attr! {
			struct $inner $( ( $($generics)* ) )? {
				$($body)*
			}
		}
	}
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
	struct ConstantValue('a) {
		constantvalue_index: CPIndex<'a, ConstantValueInfo<'a>>
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
	struct Code('a) {
		max_stack: u16,
		max_locals: u16,
//		code: &'a [u8] = (length_data!(be_u32)),
		// u32
		#[binform(read = "read_code", write = "write_code")]
		code: Vec<u8>,
		#[binform(len = "u16")]
		exception_table: Vec<Exception<'a>>,
		attributes: Attributes<'a>,
	}
}

fn read_code<I: Read, BO: ByteOrder, L>(input: &mut I) -> ReadResult<Vec<u8>> {
	let len = input.read_u32::<BO>()?;
	let mut result = Vec::with_capacity(len as usize);

	for _ in 0..len {
		result.push(<u8 as FromBytes<BO, L>>::from_bytes(input)?);
	}
	Ok(result)
}

fn write_code<O: Write, BO: ByteOrder, L>(value: &Vec<u8>, output: &mut O) -> WriteResult {
	let len = value.len();
	if len > u32::max_value() as usize {
		return Err(WriteError::TooLarge(len));
	}
	output.write_u32::<BO>(len as u32)?;
	output.write_all(value)?;
	Ok(())
}

def! {
	struct Exception('a) {
		start_pc: u16,
		end_pc: u16,
		handler_pc: u16,
		catch_type: CPIndex<'a, ClassInfo<'a>>,
	}
}

table! {
	@len = "u16";
	struct StackMapTable('a) => StackMapFrame;
}

/// The bytecode offset at which a stack map frame applies is calculated by taking the
/// value `offset_delta` specified in the frame (either explicitly or implicitly), and
/// adding `offset_delta + 1` to the bytecode offset of the previous frame, unless
/// the previous frame is the initial frame of the method. In that case, the bytecode
/// offset at which the stack map frame applies is the value offset_delta specified
/// in the frame.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum StackMapFrame<'a> {
	SameFrame(u8),
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

impl<'a> FromBytes<BigEndian, ()> for StackMapFrame<'a> {
	type Output = Self;

	fn from_bytes<I: Read>(input: &mut I) -> ReadResult<Self::Output> {
//		let tag = input.read_u8::<BigEndian>()?;

//		match tag {
//			0..=63 => {
//				
//			}
//		}

		unimplemented!()
	}
}

impl<'a> ToBytes<BigEndian, ()> for StackMapFrame<'a> {
	fn to_bytes<O: Write>(&self, output: &mut O) -> WriteResult {
		unimplemented!()
	}
}

//// @TODO Jezza - 31 Dec. 2018: Swap out these "magic" numbers with constants... 
//impl<'a> StackMapFrame<'a> {
//	named!(pub parse<StackMapFrame>, switch!(be_u8,
//		0..=63 => value!(StackMapFrame::SameFrame)
//		|
//		64..=127 => do_parse!(
//			verification_type_info: pt!(VerificationTypeInfo) >>
//			(StackMapFrame::SameLocals(verification_type_info))
//		)
//		|
//		247 => do_parse!(
//			offset: be_u16 >>
//			verification_type_info: pt!(VerificationTypeInfo) >>
//			(StackMapFrame::SameLocalsExtended {
//				offset,
//				verification_type_info
//			})
//		)
//		|
//		148..=250 => do_parse!(
//			offset_delta: be_u16 >>
//			(StackMapFrame::ChopFrame(offset_delta))
//		)
//		|
//		251 => do_parse!(
//			offset: be_u16 >>
//			(StackMapFrame::SameFrameExtended(offset))
//		)
//		|
//		252 => do_parse!(
//			offset_delta: be_u16 >>
//			locals: length_count!(value!(1), pt!(VerificationTypeInfo)) >>
//			(StackMapFrame::AppendFrame {
//				offset_delta,
//				locals,
//			})
//		)
//		|
//		253 => do_parse!(
//			offset_delta: be_u16 >>
//			locals: length_count!(value!(2), pt!(VerificationTypeInfo)) >>
//			(StackMapFrame::AppendFrame {
//				offset_delta,
//				locals,
//			})
//		)
//		|
//		254 => do_parse!(
//			offset_delta: be_u16 >>
//			locals: length_count!(value!(3), pt!(VerificationTypeInfo)) >>
//			(StackMapFrame::AppendFrame {
//				offset_delta,
//				locals,
//			})
//		)
//		|
//		255 => do_parse!(
//			offset_delta: be_u16 >>
//			locals: length_count!(be_u16, pt!(VerificationTypeInfo)) >>
//			stack: length_count!(be_u16, pt!(VerificationTypeInfo)) >>
//			(StackMapFrame::FullFrame {
//				offset_delta,
//				locals,
//				stack,
//			})
//		)
//	));
//}

const ITEM_TOP: u8 = 0;
const ITEM_INTEGER: u8 = 1;
const ITEM_FLOAT: u8 = 2;
const ITEM_DOUBLE: u8 = 3;
const ITEM_LONG: u8 = 4;
const ITEM_NULL: u8 = 5;
const ITEM_UNINITIALIZED_THIS: u8 = 6;
const ITEM_OBJECT: u8 = 7;
const ITEM_UNINITIALIZED: u8 = 8;

#[derive(Debug, Eq, PartialEq, Hash, Clone, ToBytes, FromBytes)]
#[binform(endian = "be", tag = "u8")]
pub enum VerificationTypeInfo<'a> {
	#[binform(tag = "ITEM_TOP")]
	Top,
	#[binform(tag = "ITEM_INTEGER")]
	Integer,
	#[binform(tag = "ITEM_FLOAT")]
	Float,
	#[binform(tag = "ITEM_DOUBLE")]
	Double,
	#[binform(tag = "ITEM_LONG")]
	Long,
	#[binform(tag = "ITEM_NULL")]
	Null,
	#[binform(tag = "ITEM_UNINITIALIZED_THIS")]
	UninitializedThis,
	#[binform(tag = "ITEM_OBJECT")]
	ObjectVariable(CPIndex<'a, ClassInfo<'a>>),
	#[binform(tag = "ITEM_UNINITIALIZED")]
	Uninitialized(u16),
}

attr! {
	struct Exceptions('a) {
		#[binform(len = "u16")]
		table: Vec<CPIndex<'a, ClassInfo<'a>>>
	}
}

table! {
	@len = "u16";
	struct InnerClasses('a) => struct InnerClass {
		inner_class_info_index: CPIndex<'a, ClassInfo<'a>>,
		#[binform(read = "CPIndex::read_non_zero", write = "CPIndex::write_non_zero")]
		outer_class_info_index: Option<CPIndex<'a, ClassInfo<'a>>>,
		#[binform(read = "CPIndex::read_non_zero", write = "CPIndex::write_non_zero")]
		inner_name_index: Option<CPIndex<'a, UTF8Info<'a>>>,
		inner_class_access_flags: u16
	}
}

attr! {
	struct EnclosingMethod('a) {
		class_index: CPIndex<'a, ClassInfo<'a>>,
		method_index: CPIndex<'a, NameAndTypeInfo<'a>>,
	}
}

singleton!(struct Synthetic);

attr! {
	struct Signature('a) {
		class_index: CPIndex<'a, UTF8Info<'a>>,
	}
}

attr! {
	struct SourceFile('a) {
		sourcefile_index: CPIndex<'a, UTF8Info<'a>>,
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct SourceDebugExtension<'a> {
	_marker: PhantomData<&'a ()>,
	pub data: MString
}

impl<'a> Attribute<'a> for SourceDebugExtension<'a> {
	fn from_attributes(attributes: &Attributes<'a>, cp: &ConstantPool<'a>) -> Option<Self> {
		let info = attributes.named(cp, "SourceDebugExtension")?;
		let data = unsafe { MString::from_mutf8_unchecked(info.info) };
		Some(SourceDebugExtension {
			data
		})
	}
}

table! {
	@len = "u16";
	struct LineNumberTable => struct LineNumber {
		start_pc: u16,
		line_number: u16,
	}
}

table! {
	@len = "u16";
	struct LocalVariableTable('a) => struct LocalVariable {
		start_pc: u16,
		length: u16,
		name_index: CPIndex<'a, UTF8Info<'a>>,
		descriptor_index: CPIndex<'a, UTF8Info<'a>>,
		index: u16,
	}
}

table! {
	@len = "u16";
	struct LocalVariableTypeTable('a) => struct LocalVariableType {
		start_pc: u16,
		length: u16,
		name_index: CPIndex<'a, UTF8Info<'a>>,
		signature_index: CPIndex<'a, UTF8Info<'a>>,
		index: u16,
	}
}

singleton!(struct Deprecated);

table! {
	@len = "u16";
	struct RuntimeVisibleAnnotations('a) => Annotation;
}

table! {
	@len = "u16";
	struct RuntimeInvisibleAnnotations('a) => Annotation;
}

def! {
	struct Annotation('a) {
		type_index: CPIndex<'a, UTF8Info<'a>>,
		#[binform(len = "u16")]
		element_value_pairs: Vec<ElementValuePair<'a>>,
	}
}

def! {
	struct ElementValuePair('a) {
		element_name_index: CPIndex<'a, UTF8Info<'a>>,
//		element_value: ElementValue<'a>,
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, ToBytes, FromBytes)]
#[binform(endian = "be", tag = "u8")]
pub enum ElementValue<'a> {
	#[binform(tag = "0")]
	Byte(PhantomData<&'a ()>)
}

//parser! {
//	enum ElementValue<'a> = (kind: be_u8) {
//		Byte(b'B') {
//			const_value_index: CPIndex<'a, IntegerInfo> = (pt!(CPIndex))
//		},
//		Char(b'C') {
//			const_value_index: CPIndex<'a, IntegerInfo> = (pt!(CPIndex))
//		},
//		Double(b'D') {
//			const_value_index: CPIndex<'a, DoubleInfo> = (pt!(CPIndex))
//		},
//		Float(b'F') {
//			const_value_index: CPIndex<'a, FloatInfo> = (pt!(CPIndex))
//		},
//		Integer(b'I') {
//			const_value_index: CPIndex<'a, IntegerInfo> = (pt!(CPIndex))
//		},
//		Long(b'J') {
//			const_value_index: CPIndex<'a, LongInfo> = (pt!(CPIndex))
//		},
//		Short(b'S') {
//			const_value_index: CPIndex<'a, IntegerInfo> = (pt!(CPIndex))
//		},
//		Boolean(b'Z') {
//			const_value_index: CPIndex<'a, IntegerInfo> = (pt!(CPIndex))
//		},
//		String(b's') {
//			const_value_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
//		},
//		Enum(b'e') {
//			type_name_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex)),
//			const_name_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
//		},
//		Class(b'c') {
//			class_info_index: CPIndex<'a, UTF8Info<'a>> = (pt!(CPIndex))
//		},
//		Annotation(b'@') {
//			annotation_value: Annotation<'a> = (pt!(Annotation))
//		},
//		Array(b'[') {
//			array_value: Vec<ElementValue<'a>> = (length_count!(be_u16, pt!(ElementValue)))
//		},
//	}
//}

table! {
	@len = "u8";
	struct RuntimeVisibleParameterAnnotations('a) => ParameterAnnotations;
}

table! {
	@len = "u8";
	struct RuntimeInvisibleParameterAnnotations('a) => ParameterAnnotations;
}

def! {
	struct ParameterAnnotations('a) {
		#[binform(len = "u16")]
		annotations: Vec<Annotation<'a>>
	}
}

table! {
	@len = "u16";
	struct RuntimeVisibleTypeAnnotations('a) => TypeAnnotation;
}

table! {
	@len = "u16";
	struct RuntimeInvisibleTypeAnnotations('a) => TypeAnnotation;
}

def! {
	struct TypeAnnotation('a) {
		target_info         : TargetInfo,
		target_path         : TypePath,
		type_index          : CPIndex<'a, UTF8Info<'a>>,
		#[binform(len = "u16")]
		element_value_pairs : Vec<ElementValuePair<'a>>
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, ToBytes, FromBytes)]
#[binform(endian = "be", tag = "u8")]
pub enum TargetInfo {
	#[binform(tag = "1")]
	Unknown
//	TypeParameter(u8),
//	SuperType(u16),
//	TypeParameterBound {
//		type_parameter_index: u8,
//		bound_index: u8,
//	},
//	Empty,
//	FormalParameter(u8),
//	Throws(u16),
//	LocalVar {
//		start_pc: u16,
//		length: u16,
//		index: u16,
//	},
//	Catch(u16),
//	Offset(u16),
//	TypeArgument {
//		offset: u16,
//		type_argument_index: u8
//	},
}
//
//impl TargetInfo {
//	named!(pub parse<TargetInfo>, switch!(be_u8,
//		0x00 | 0x01 => do_parse!(
//			index: be_u8 >>
//			(TargetInfo::TypeParameter(index))
//		)
//		|
//		0x10 => do_parse!(
//			index: be_u16 >>
//			(TargetInfo::SuperType(index))
//		)
//		|
//		0x11 | 0x12 => do_parse!(
//			type_parameter_index: be_u8 >>
//			bound_index: be_u8 >>
//			(TargetInfo::TypeParameterBound {
//				type_parameter_index,
//				bound_index,
//			})
//		)
//		|
//		0x13 | 0x14 | 0x15 => value!(TargetInfo::Empty)
//		|
//		0x16 => do_parse!(
//			index: be_u8 >>
//			(TargetInfo::FormalParameter(index))
//		)
//		|
//		0x17 => do_parse!(
//			throws_type_index: be_u16 >>
//			(TargetInfo::Throws(throws_type_index))
//		)
//		|
//		0x40 | 0x41 => do_parse!(
//			start_pc: be_u16 >>
//			length: be_u16 >>
//			index: be_u16 >>
//			(TargetInfo::LocalVar {
//				start_pc,
//				length,
//				index,
//			})
//		)
//		|
//		0x42 => do_parse!(
//			exception_table_index: be_u16 >>
//			(TargetInfo::Catch(exception_table_index))
//		)
//		|
//		0x43 | 0x44 | 0x45 | 0x46 => do_parse!(
//			offset: be_u16 >>
//			(TargetInfo::Offset(offset))
//		)
//		|
//		0x47 | 0x48 | 0x49 | 0x4A | 0x4B => do_parse!(
//			offset: be_u16 >>
//			type_argument_index: be_u8 >>
//			(TargetInfo::TypeArgument {
//				offset,
//				type_argument_index,
//			})
//		)
//	));
//}

def! {
	struct TypePath {
		#[binform(len = "u8")]
		data: Vec<TypePathSegment>
	}
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, ToBytes, FromBytes)]
#[binform(endian = "be", tag = "u8")]
pub enum TypePathSegment {
	#[binform(tag = "0", after(expect(ty = "u8", value = "0")))]
	Array,
	#[binform(tag = "1", after(expect(ty = "u8", value = "0")))]
	NestedType,
	#[binform(tag = "3", after(expect(ty = "u8", value = "0")))]
	WildcardBound,
	#[binform(tag = "4")]
	TypeArgument(u8),
}

attr! {
	struct AnnotationDefault('a) {
		default_value: ElementValue<'a>
	}
}

table! {
	@len = "u16";
	struct BootstrapMethods('a) => struct BootstrapMethod {
		bootstrap_method_ref: CPIndex<'a, MethodHandleInfo<'a>>,
		#[binform(len = "u16")]
		bootstrap_arguments: Vec<CPIndex<'a, LoadableConstant<'a>>>,
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
	@len = "u16";
	struct MethodParameters('a) => MethodParameter;
}

def! {
	struct MethodParameter('a) {
		#[binform(read = "CPIndex::read_non_zero", write = "CPIndex::write_non_zero")]
		name_index: Option<CPIndex<'a, UTF8Info<'a>>>,
		access_flags: u16
	}
}

attr! {
	struct Module('a) {
		module_name_index: CPIndex<'a, ModuleInfo<'a>>,
		module_flags: u16,
		#[binform(read = "CPIndex::read_non_zero", write = "CPIndex::write_non_zero")]
		module_version_index : Option<CPIndex<'a, ModuleInfo<'a>>>,
		#[binform(len = "u16")]
		requires: Vec<Requires<'a>>,
		#[binform(len = "u16")]
		exports: Vec<Exports<'a>>,
		#[binform(len = "u16")]
		opens: Vec<Opens<'a>>,
		#[binform(len = "u16")]
		uses: Vec<CPIndex<'a, ClassInfo<'a>>>,
		#[binform(len = "u16")]
		provides: Vec<Provides<'a>>
	}
}

def! {
	struct Requires('a) {
		requires_index: CPIndex<'a, ModuleInfo<'a>>,
		requires_flags: u16,
		#[binform(read = "CPIndex::read_non_zero", write = "CPIndex::write_non_zero")]
		requires_version_index: Option<CPIndex<'a, UTF8Info<'a>>>,
	}
}

def! {
	struct Exports('a) {
		exports_index: CPIndex<'a, PackageInfo<'a>>,
		exports_flags: u16,
		#[binform(len = "u16")]
		exports_to: Vec<CPIndex<'a, ModuleInfo<'a>>>
	}
}

def! {
	struct Opens('a) {
		opens_index: CPIndex<'a, PackageInfo<'a>>,
		opens_flags: u16,
		#[binform(len = "u16")]
		opens_to: Vec<CPIndex<'a, ModuleInfo<'a>>>
	}
}

def! {
	struct Provides('a) {
		provides: CPIndex<'a, ClassInfo<'a>>,
		#[binform(len = "u16")]
		provides_with: Vec<CPIndex<'a, ClassInfo<'a>>>
	}
}

attr! {
	struct ModulePackages('a) {
		#[binform(len = "u16")]
		packages: Vec<CPIndex<'a, PackageInfo<'a>>>
	}
}

attr! {
	struct ModuleMainClass('a) {
		main_class_index: CPIndex<'a, ClassInfo<'a>>
	}
}

attr! {
	struct NestHost('a) {
		host_class_index: CPIndex<'a, ClassInfo<'a>>
	}
}

attr! {
	struct NestMembers('a) {
		#[binform(len = "u16")]
		classes: Vec<CPIndex<'a, ClassInfo<'a>>>
	}
}
