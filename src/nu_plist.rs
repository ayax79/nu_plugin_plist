use std::time::SystemTime;

use chrono::{DateTime, FixedOffset, Offset, Utc};
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{record, Category, PluginExample, PluginSignature, Span, Value as NuValue};
use plist::{Date as PlistDate, Dictionary, Value as PlistValue};

pub struct NuPlist;

impl Plugin for NuPlist {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![PluginSignature::build("from plist")
            .usage("Parse text as an Apple plist document")
            .plugin_examples(vec![PluginExample {
                example: "cat file.plist | from plist".to_string(),
                description: "Convert a plist file to a table".to_string(),
                result: None,
            }])
            .category(Category::Experimental)]
    }

    fn run(
        &mut self,
        _name: &str,
        call: &EvaluatedCall,
        input: &NuValue,
    ) -> Result<NuValue, LabeledError> {
        match input {
            NuValue::String { val, internal_span } => {
                let plist = plist::from_bytes(val.as_bytes())
                    .map_err(|e| build_label_error(format!("{}", e), internal_span))?;
                let converted = convert_plist_value(&plist, call.head);
                Ok(converted)
            }
            _ => Err(build_label_error(
                format!("Invalid input, must be string not: {:?}", input),
                &call.head,
            )),
        }
    }
}

fn build_label_error(msg: String, span: &Span) -> LabeledError {
    LabeledError {
        label: "ERROR from plugin".to_string(),
        msg,
        span: Some(span.to_owned()),
    }
}

fn convert_plist_value(plist_val: &PlistValue, span: Span) -> NuValue {
    match plist_val {
        PlistValue::String(s) => NuValue::string(s.to_owned(), span),
        PlistValue::Boolean(b) => NuValue::bool(b.to_owned(), span),
        PlistValue::Real(r) => NuValue::float(r.to_owned(), span),
        PlistValue::Date(d) => NuValue::date(convert_date(d), span),
        PlistValue::Integer(i) => NuValue::int(i.as_signed().unwrap(), span),
        PlistValue::Uid(uid) => NuValue::float(f64::from_bits(uid.get()), span),
        PlistValue::Data(data) => NuValue::binary(data.to_owned(), span),
        PlistValue::Array(arr) => NuValue::list(convert_array(arr, span), span),
        PlistValue::Dictionary(dict) => convert_dict(dict, span),
        _ => NuValue::nothing(span),
    }
}

fn convert_dict(dict: &Dictionary, span: Span) -> NuValue {
    let mut result = record!();
    for i in dict.into_iter() {
        result.push(i.0, convert_plist_value(i.1, span))
    }
    NuValue::record(result, span)
}

fn convert_array(plist_array: &Vec<PlistValue>, span: Span) -> Vec<NuValue> {
    plist_array
        .iter()
        .map(|item| convert_plist_value(item, span))
        .collect()
}

pub fn convert_date(plist_date: &PlistDate) -> DateTime<FixedOffset> {
    // In the docs the plist date object is listed as a utc timestamp, so this
    // conversion shoould be fine
    let plist_sys_time: SystemTime = plist_date.to_owned().into();
    let utc_date: DateTime<Utc> = plist_sys_time.into();
    let utc_offset = utc_date.offset().fix();
    utc_date.with_timezone(&utc_offset)
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::Datelike;
    use std::time::SystemTime;

    #[test]
    fn test_convert_date() {
        let epoch = SystemTime::UNIX_EPOCH;
        let plist_date = epoch.into();

        let datetime = convert_date(&plist_date);
        assert_eq!(1970, datetime.year());
        assert_eq!(1, datetime.month());
        assert_eq!(1, datetime.day());
    }
}
