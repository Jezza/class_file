//use nom::IResult;
//pub trait Parser<'a>: Sized {
//	fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self, u32>;
//}

macro_rules! parser {
	(struct $type:ident $(<$first:lifetime $(,$rest:lifetime)*>)? {
		$($field_name:ident: $field_type:ty = ($($field_parser:tt)*)),* $(,)?
	}) => {
		#[derive(Debug, Eq, PartialEq, Hash, Clone)]
		pub struct $type $(<$first $(,$rest)*>)? {
			$(pub $field_name: $field_type),*
		}

		impl $(<$first $(,$rest)*>)? $type $(<$first $(,$rest)*>)? {
			named!(pub parse<$type>, do_parse!(
				$($field_name: $($field_parser)* >>)*
				($type {
					$($field_name),*
				})
			));
		}
	};
	(enum $type:ident $(<$first:lifetime $(,$rest:lifetime)*>)? = ($ident:ident: $($tag_parser:tt)*) {
		$(
			$variant:ident ($($variant_tag:tt)*) {
				$($field_name:ident: $field_type:ty = ($($field_parser:tt)*)),* $(,)?
			}
		),* $(,)?
	}) => {
		#[derive(Debug, Eq, PartialEq, Hash, Clone)]
		pub enum $type $(<$first $(,$rest)*>)? {
			$(
				$variant {
					$($field_name: $field_type),*
				},
			)*
		}

		impl $(<$first $(,$rest)*>)? $type $(<$first $(,$rest)*>)? {
			named!(pub parse<$type>, do_parse!(
				$ident: $($tag_parser)* >>
				result: switch!(value!($ident),
					$(
						$($variant_tag)* => do_parse!(
							$($field_name: $($field_parser)* >>)*
							($type::$variant {
								$($field_name,)*
							})
						)
					)|*
				) >>
				(result)
			));
		}
	};
}

macro_rules! pt {
//	($i:expr,$t:ident) => (<$t as $crate::parsing::Parser>::parse($i));
	($i:expr,$t:ident) => ($t::parse($i));
}