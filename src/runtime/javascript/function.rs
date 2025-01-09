use std::{
    io::{Read as _, Write as _},
    time::Duration,
};

use boa_engine::{
    job::NativeJob,
    object::builtins::{JsPromise, JsUint8Array},
    value::Type,
    Context, JsObject, JsResult, JsValue,
};
use bytes::Bytes;

use crate::runtime::{
    javascript::host_defined::{HostDefined, UserInput, UserOutput},
    RuntimeRequest,
};

use super::class::ByteData;

pub fn get_user_input(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    UserInput::extract(context.realm())
        .map(|input| {
            let array = JsUint8Array::from_iter(input.data, context)?;
            Ok(JsValue::from(array))
            // let data = ByteData { inner: input.data };
            // Ok(JsValue::from(JsObject::from_proto_and_data(None, data)))
        })
        .unwrap_or_else(|| Ok(JsValue::undefined()))
}

pub fn set_user_output(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data_js_obj = args.first().expect("data is required");

    let data = match data_js_obj.get_type() {
        Type::Undefined | Type::Null => None,
        _ => {
            let data = data_js_obj.to_object(context)?;
            let data: Bytes = JsUint8Array::from_object(data)?.iter(context).collect();

            Some(data)
        }
    };

    UserOutput { data }.insert(context.realm());

    Ok(JsValue::undefined())
}

pub fn sleep(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    // let callable = match args.first() {
    //     Some(callback) => match callback.as_callable() {
    //         Some(callback) => callback.clone(),
    //         None => return Err(JsError::from_native(JsNativeError::ERROR)),
    //     },
    //     None => return Err(JsError::from_native(JsNativeError::ERROR)),
    // };

    // let ms = match args.get(1) {
    //     Some(ms) => ms.to_number(context)? as u64,
    //     None => 0,
    // };

    let ms = match args.first() {
        Some(ms) => ms.to_number(context)? as u64,
        None => 0,
    };

    // Create a request object and insert it into the context
    RuntimeRequest::Sleep(Duration::from_millis(ms)).insert(context.realm());
    tracing::trace!("request inserted into context");

    let (promise, resolver) = JsPromise::new_pending(context);

    let job = NativeJob::new(move |context| {
        tracing::trace!("promise job called");

        // let _ = RuntimeResponse::extract(context.realm()).unwrap();
        // let _ = callable.call(&JsValue::undefined(), &[], context).unwrap();

        resolver
            .resolve
            .call(&JsValue::undefined(), &[JsValue::undefined()], context)
    });

    context.job_queue().enqueue_promise_job(job, context);

    Ok(JsValue::from(promise))
}

pub fn blocking_sleep(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let ms = match args.first() {
        Some(ms) => ms.to_number(context)? as u64,
        None => 0,
    };

    std::thread::sleep(Duration::from_millis(ms));

    Ok(JsValue::undefined())
}

pub fn yield_now(_this: &JsValue, _args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    // Create a request object and insert it into the context
    RuntimeRequest::Yield.insert(context.realm());
    tracing::trace!("request inserted into context");

    let (promise, resolver) = JsPromise::new_pending(context);

    let job = NativeJob::new(move |context| {
        tracing::trace!("promise job called");
        resolver
            .resolve
            .call(&JsValue::undefined(), &[JsValue::undefined()], context)
    });

    context.job_queue().enqueue_promise_job(job, context);

    Ok(JsValue::from(promise))
}

pub fn compress(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let input_obj = args.first().unwrap().to_object(context).unwrap();

    // let start = std::time::Instant::now();
    let data = JsUint8Array::from_object(input_obj)?
        .iter(context)
        .collect::<Bytes>();
    // let finished = start.elapsed();
    // println!("compress");
    // println!("data collection finished: {:?}", finished);

    // let data = UserInput::extract(context.realm()).unwrap().data;

    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());

    e.write_all(&data).unwrap();
    let compressed = e.finish().unwrap();
    let array = JsUint8Array::from_iter(compressed, context)?;

    Ok(JsValue::from(array))
    // UserOutput {
    //     data: Some(compressed.into()),
    // }
    // .insert(context.realm());

    // Ok(JsValue::undefined())
}

pub fn resize(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let input_obj = args.first().unwrap().to_object(context).unwrap();
    let data = JsUint8Array::from_object(input_obj)?
        .iter(context)
        .collect::<Bytes>();

    // let data = UserInput::extract(context.realm()).unwrap().data;

    // create zip::ZipArchive from data
    let mut input_zip = zip::ZipArchive::new(std::io::Cursor::new(data)).unwrap();
    let mut output_zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    for i in 0..input_zip.len() {
        let mut file = input_zip.by_index(i).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();
        let image = image::load_from_memory(&contents).unwrap();
        let resized = image.resize(640, 360, image::imageops::FilterType::Gaussian);

        output_zip
            .start_file(format!("image_{i}.jpg"), options)
            .unwrap();
        output_zip.write_all(resized.as_bytes()).unwrap();
    }

    let output = output_zip.finish().unwrap().into_inner();
    let array = JsUint8Array::from_iter(output.into_iter(), context)?;

    Ok(JsValue::from(array))
    // UserOutput {
    //     data: Some(output.into()),
    // }
    // .insert(context.realm());

    // Ok(JsValue::undefined())
}
