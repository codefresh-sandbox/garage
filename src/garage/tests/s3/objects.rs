use crate::common;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};

const STD_KEY: &str = "hello world";
const CTRL_KEY: &str = "\x00\x01\x02\x00";
const UTF8_KEY: &str = "\u{211D}\u{1F923}\u{1F44B}";
const BODY: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

#[tokio::test]
async fn test_putobject() {
	let ctx = common::context();
	let bucket = ctx.create_bucket("putobject");

	{
		// Send an empty object (can serve as a directory marker)
		// with a content type
		let etag = "\"d41d8cd98f00b204e9800998ecf8427e\"";
		let content_type = "text/csv";
		let r = ctx
			.client
			.put_object()
			.bucket(&bucket)
			.key(STD_KEY)
			.content_type(content_type)
			.send()
			.await
			.unwrap();

		assert_eq!(r.e_tag.unwrap().as_str(), etag);
		// We return a version ID here
		// We should check if Amazon is returning one when versioning is not enabled
		assert!(r.version_id.is_some());

		let _version = r.version_id.unwrap();

		let o = ctx
			.client
			.get_object()
			.bucket(&bucket)
			.key(STD_KEY)
			.send()
			.await
			.unwrap();

		assert_bytes_eq!(o.body, b"");
		assert_eq!(o.e_tag.unwrap(), etag);
		// We do not return version ID
		// We should check if Amazon is returning one when versioning is not enabled
		// assert_eq!(o.version_id.unwrap(), _version);
		assert_eq!(o.content_type.unwrap(), content_type);
		assert!(o.last_modified.is_some());
		assert_eq!(o.content_length.unwrap(), 0);
		assert_eq!(o.parts_count, None);
		assert_eq!(o.tag_count, None);
	}

	{
		// Key with control characters,
		// no content type and some data
		let etag = "\"49f68a5c8493ec2c0bf489821c21fc3b\"";
		let data = ByteStream::from_static(b"hi");

		let r = ctx
			.client
			.put_object()
			.bucket(&bucket)
			.key(CTRL_KEY)
			.body(data)
			.send()
			.await
			.unwrap();

		assert_eq!(r.e_tag.unwrap().as_str(), etag);
		assert!(r.version_id.is_some());

		let o = ctx
			.client
			.get_object()
			.bucket(&bucket)
			.key(CTRL_KEY)
			.send()
			.await
			.unwrap();

		assert_bytes_eq!(o.body, b"hi");
		assert_eq!(o.e_tag.unwrap(), etag);
		assert!(o.last_modified.is_some());
		assert_eq!(o.content_length.unwrap(), 2);
		assert_eq!(o.parts_count, None);
		assert_eq!(o.tag_count, None);
	}

	{
		// Key with UTF8 codepoints including emoji
		let etag = "\"d41d8cd98f00b204e9800998ecf8427e\"";

		let r = ctx
			.client
			.put_object()
			.bucket(&bucket)
			.key(UTF8_KEY)
			.send()
			.await
			.unwrap();

		assert_eq!(r.e_tag.unwrap().as_str(), etag);
		assert!(r.version_id.is_some());

		let o = ctx
			.client
			.get_object()
			.bucket(&bucket)
			.key(UTF8_KEY)
			.send()
			.await
			.unwrap();

		assert_bytes_eq!(o.body, b"");
		assert_eq!(o.e_tag.unwrap(), etag);
		assert!(o.last_modified.is_some());
		assert_eq!(o.content_length.unwrap(), 0);
		assert_eq!(o.parts_count, None);
		assert_eq!(o.tag_count, None);
	}
}

#[tokio::test]
async fn test_getobject() {
	let ctx = common::context();
	let bucket = ctx.create_bucket("getobject");

	let etag = "\"46cf18a9b447991b450cad3facf5937e\"";
	let data = ByteStream::from_static(BODY);

	let r = ctx
		.client
		.put_object()
		.bucket(&bucket)
		.key(STD_KEY)
		.body(data)
		.send()
		.await
		.unwrap();

	assert_eq!(r.e_tag.unwrap().as_str(), etag);

	{
		let o = ctx
			.client
			.get_object()
			.bucket(&bucket)
			.key(STD_KEY)
			.range("bytes=1-9")
			.send()
			.await
			.unwrap();

		assert_eq!(o.content_range.unwrap().as_str(), "bytes 1-9/62");
		assert_bytes_eq!(o.body, &BODY[1..10]);
	}
	{
		let o = ctx
			.client
			.get_object()
			.bucket(&bucket)
			.key(STD_KEY)
			.range("bytes=9-")
			.send()
			.await
			.unwrap();
		assert_eq!(o.content_range.unwrap().as_str(), "bytes 9-61/62");
		assert_bytes_eq!(o.body, &BODY[9..]);
	}
	{
		let o = ctx
			.client
			.get_object()
			.bucket(&bucket)
			.key(STD_KEY)
			.range("bytes=-5")
			.send()
			.await
			.unwrap();
		assert_eq!(o.content_range.unwrap().as_str(), "bytes 57-61/62");
		assert_bytes_eq!(o.body, &BODY[57..]);
	}
	{
		let exp = aws_sdk_s3::primitives::DateTime::from_secs(10000000000);
		let o = ctx
			.client
			.get_object()
			.bucket(&bucket)
			.key(STD_KEY)
			.response_content_type("application/x-dummy-test")
			.response_cache_control("ccdummy")
			.response_content_disposition("cddummy")
			.response_content_encoding("cedummy")
			.response_content_language("cldummy")
			.response_expires(exp)
			.send()
			.await
			.unwrap();
		assert_eq!(o.content_type.unwrap().as_str(), "application/x-dummy-test");
		assert_eq!(o.cache_control.unwrap().as_str(), "ccdummy");
		assert_eq!(o.content_disposition.unwrap().as_str(), "cddummy");
		assert_eq!(o.content_encoding.unwrap().as_str(), "cedummy");
		assert_eq!(o.content_language.unwrap().as_str(), "cldummy");
		assert_eq!(o.expires.unwrap(), exp);
		assert_bytes_eq!(o.body, &BODY[..]);
	}
}

#[tokio::test]
async fn test_deleteobject() {
	let ctx = common::context();
	let bucket = ctx.create_bucket("deleteobject");

	let mut to_del = Delete::builder();

	// add content without data
	for i in 0..5 {
		let k = format!("k-{}", i);
		ctx.client
			.put_object()
			.bucket(&bucket)
			.key(k.to_string())
			.send()
			.await
			.unwrap();
		if i > 0 {
			to_del = to_del.objects(ObjectIdentifier::builder().key(k).build().unwrap());
		}
	}

	// add content with data
	for i in 0..5 {
		let k = format!("l-{}", i);
		let data = ByteStream::from_static(BODY);
		ctx.client
			.put_object()
			.bucket(&bucket)
			.key(k.to_string())
			.body(data)
			.send()
			.await
			.unwrap();

		if i > 0 {
			to_del = to_del.objects(ObjectIdentifier::builder().key(k).build().unwrap());
		}
	}

	ctx.client
		.delete_object()
		.bucket(&bucket)
		.key("k-0")
		.send()
		.await
		.unwrap();

	ctx.client
		.delete_object()
		.bucket(&bucket)
		.key("l-0")
		.send()
		.await
		.unwrap();

	let r = ctx
		.client
		.delete_objects()
		.bucket(&bucket)
		.delete(to_del.build().unwrap())
		.send()
		.await
		.unwrap();

	assert_eq!(r.deleted.unwrap().len(), 8);

	let l = ctx
		.client
		.list_objects_v2()
		.bucket(&bucket)
		.send()
		.await
		.unwrap();

	assert!(l.contents.is_none());

	// Deleting a non-existing object shouldn't be a problem
	ctx.client
		.delete_object()
		.bucket(&bucket)
		.key("l-0")
		.send()
		.await
		.unwrap();
}
