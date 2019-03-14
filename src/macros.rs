#[macro_export]
macro_rules! def {
    (
    	$(#[$struct_attr:meta])*
		struct $type:ident $( ( $($generics:tt),* ) )? {
			$(
				$(#[$field_attr:meta])*
				$field_name:ident: $field_type:ty
				$(,)?
			)*
		} 
    ) => {
    	#[derive(Debug, Eq, PartialEq, Hash, Clone, ToBytes, FromBytes)]
		#[binform(endian = "be")]
		$(#[$struct_attr])*
		pub struct $type $( < $($generics),* > )? {
			$(
				$(#[$field_attr])*
				pub $field_name: $field_type,
			)*
			
		}
    };
    (
		$(#[$enum_attr:meta])*
		enum $type:ident $( ( $($generics:tt),* ) )? {
			$($body:tt)*
		}
    ) => {
    	
    }
}

#[macro_export]
macro_rules! def_enum_of_structs {
    (
    	$(#[$enum_attr:meta])*
    	enum $name:ident $( ( $($generics:tt)* ) )? {
    		$(
    			$(#[$variant_self_attr:meta])*
    			$(@[$variant_pass_attr:meta])*
    			$variant:ident (
    				$(#$aux:ident)? $struct:ident $( ( $($struct_generics:tt)* ) )?
					$(
						{
							$($body:tt)*
						}
					)?
    			)
    		),*
    	}
    ) => {
    	#[derive(Debug, Eq, PartialEq, Hash, Clone, ToBytes, FromBytes)]
   		$(#[$enum_attr])*
		pub enum $name $( < $($generics)* > )? {
			$(
				$(#[$variant_self_attr])*
    			$variant($struct $( < $($struct_generics)* > )?)
    		),*
		}

		$(
			def_enum_of_structs!(
				$(#$aux)?
				@attr;
				$(#[$variant_pass_attr])*
				@variant;
				$variant
				@decl;
				$struct $( ( $($struct_generics)* ) )?
				@body;
				$(
					{
						$($body)*
					}
				)?
			);
		)*
    };
	(
		@attr;
		$(#[$attr:meta])*
		@variant;
		$variant:ident
		@decl;
		$struct:ident $( ( $($struct_generics:tt)* ) )?
		@body;
		$(
			{
				$($body:tt)*
			}
		)?
	) => {
		#[derive(Debug, Eq, PartialEq, Hash, Clone, ToBytes, FromBytes)]
		$(#[$attr])*
		pub struct $struct $( < $($struct_generics)* > )? {
			$($($body)*)?
		}

		def_fetch!($struct $( ( $($struct_generics)* ) )? => $variant);
	};
	(
		#use
		$($rest:tt)*
	) => {}
}