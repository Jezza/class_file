extern crate class_file;

use std::io::Cursor;

use class_file::*;
use class_file::ops::*;

#[test]
fn passthrough() {
	let class_data = include_bytes!("Version55.class");

	let mut input = class_data.to_vec();
	let mut input_cursor = Cursor::new(&mut input);

	let class_file = ClassFile::open(&mut input_cursor)
		.expect("Failed to parse input.");

	let mut output = vec![];
	let mut output_cursor = Cursor::new(&mut output);

	class_file.to_bytes(&mut output_cursor).unwrap();

	assert_eq!(input, output);
}

#[test]
fn load_class() {
	let data = include_bytes!("Version55.class");
	let mut data = data.to_vec();
	let mut input = Cursor::new(&mut data);
	let class_file = ClassFile::open(&mut input)
		.expect("Failed to parse \"Version55.class\"");
	assert_eq!(class_file.major_version, 55);
	assert_eq!(class_file.minor_version, 0);
	assert_eq!(class_file.access_flags, PUBLIC | SUPER);

	let cp = &class_file.constant_pool;
	assert_eq!(cp.entries.len(), 12);
	for value in cp {
		println!("{:?}", value);
	}

	let class_name: &mstr = {
		let this_class = class_file.this_class;
		let class_info = cp.index(this_class)
			.expect("Unable to locate \"this_class\" inside of constant pool.");
		let info = cp.index(class_info.name_index)
			.expect("Unable to locate \"this_class.name_index\" in constant pool");
		&info.data
	};
	// The name of the class itself.
	assert_eq!(class_name.to_utf8(), "Version55");

	let super_class: &mstr = {
		let super_class = class_file.super_class;
		let class_info = cp.index(super_class)
			.expect("Unable to locate \"super_class\" inside of constant pool.");
		let info = cp.index(class_info.name_index)
			.expect("Unable to locate \"super_class.name_index\" in constant pool");
		&info.data
	};
	// Default for classes that don't specify a super.
	assert_eq!(super_class.to_utf8(), "java/lang/Object");

	// No interfaces were implemented.
	assert_eq!(class_file.interfaces.len(), 0);

	// No fields, obviously...
	assert_eq!(class_file.fields.len(), 0);

	// Remember, default constructor!
	assert_eq!(class_file.methods.len(), 1);

	let mut output = vec![];
	let mut output_cursor = Cursor::new(&mut output);

	class_file.to_bytes(&mut output_cursor).unwrap();

	let data_len = data.len();
	println!("{}", data_len);
	let len =  output.len();
	println!("{}", len);
	assert_eq!(data, output);
}
