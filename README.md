A library for parsing JVM classfiles.  
It uses nom to achieve its amazing speed, so go thank the author.    

As of writing, this crate is "basically feature complete".  

 * All standard attributes as declared within JVMS 11.  
 * All standard constant pool entries as declared within JVMS 11.  
 * Type checked constant pool indices  

There's not much to explain about the implementation.  
It uses nom and a lot of macros...  
And not all from nom...  

This crate should be a complete one-to-one implementation of the spec.

If the spec says some binary structure defines this structure which defines this one, and so one.  
Then in here, it'll be replicated.  
This crate is designed not as a user facing library, but to be used by other libraries.  

It's very low level.

There's a couple things to note:

ConstantPool indices carry information about their contents.  
So, if you get back an index into the constant pool, you will get back a specific value.  

A really easy example to show it off is:

```rust
extern crate class_file;

use class_file::*;

fn main() {
	let data = include_bytes!("Main.class");
	let class_file = ClassFile::parse(data)
    		.expect("Failed to parse \"Main.class\"")
    		.1; // This is needed because I actually return a nom result, which means you'll get back the remaining bytes.... Still undecided if I should hide that...

	let cp = &class_file.constant_pool;

	// So if you don't know, this_class is an index into the constant pool.
	// Specifically, it's an index to a ClassInfo at that location.
	// So the type of this_class is something like `CPIndex<'_, ClassInfo<'_>>`
	let this_class = class_file.this_class;

	// This information is used when you try fetching data from the constant pool.

	// This returns a ClassInfo, and from there, you'll start going down the rabbit hole that is the specification.
	let class_info = cp.index(this_class);

	// For example, to fetch the name of the class itself, you have to do something like:
	let class_name: &mstr = {
		let this_class = class_file.this_class;
		let class_info = cp.index(this_class)
			.expect("Unable to locate \"this_class\" inside of constant pool.");
		let info = cp.index(class_info.name_index)
			.expect("Unable to locate \"this_class.name_index\" in constant pool");
		info.data
	};

	// But as you might have noticed, that's not a str, but a mstr.
	// This is because the JVM classfile uses MUTF8 strings.
	// So to save converting literally every string a classfile has, it's returned as a mstr.
	// You can easily call .to_utf8() on it, and it'll convert it to a `Cow<'_, str>`.
	println!("This class: {}", class_name.to_utf8());
	// Side-note: mstr doesn't implement Display, because I'm a lazy fuck, but I'll get around to it...
}
```

If you're interested in a bit more, go take a look at the one test I have...  
Although, saying that, it's basically the same as the code above...  

A quite note:  
Currently, a couple of things are pub until I expand more of the internals and ergonomics of this crate...

What other crazy things does this crate do...

Uh, it has constants for most things in the JVM.  
Most things...  
There's some things that are missing, and I will add them in time.  
I know, for example, there are some constants in the attr module that I haven't moved into there yet...  

Uh, what else...

Attributes are just a trait thing, so you can implement your own Attribute.  
It's pretty easy if you use a macro.  
Actually, even if you don't, it's still pretty easy.  
You just need to map nom's result to an optional.  
And actually implement the parsing part.  

Goals
---

MUTF-8 handling isn't perfect.... I want to expand that and make it that much more seamless.

This is probably more crazy, but ideally, I want to be able to serialise the entire structure.  
The idea would be you can use this library to parse a class file, modify it, and then dump it back.  
Or even more crazy, would be able to create these structures from scratch.  
I have a couple of ideas with regard to that.  

I was thinking of an objectasm-like library, and while that's still an option, I don't know if I like how it turned out.  
The visitor pattern played weirdly with Rust's semantics.  
The end result was it got really messy trying to what you'd normally use objectasm for.  

So, I think after I clean this and mutf8 up, I want to try another idea.  
Basically, using the same techniques that the `syn` and `quote` crates have, create some macro library that accepts some syntax for creating and transforming classfiles.  

Jasmin falls so well into this category that it makes it a perfect target.  
I'll look into that and see how it goes.  
Ideally, I would want to combine this and this theoretical jasmin crate to create and modify classfiles with ease.

* [ ] Improve MUTF-8 handling  
* [ ] Start utilising this crate, perhaps with a quote-like crate with jasmin-like syntax.  

I'll probably forget to change this list, but as of now, those are my main goals.
