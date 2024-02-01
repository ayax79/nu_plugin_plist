use std::time::SystemTime;

use chrono::{DateTime, FixedOffset, Offset, Utc};
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, PluginExample, PluginSignature, Record, Span, Type, Value as NuValue};
use plist::{Date as PlistDate, Dictionary, Value as PlistValue};

pub struct NuPlist;

impl Plugin for NuPlist {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![PluginSignature::build("from plist")
            .input_output_types(vec![(Type::String, Type::Any)])
            .usage("Parse text as an Apple plist document")
            .plugin_examples(vec![PluginExample {
                example: "cat file.plist | from plist".to_string(),
                description: "Convert a plist file to a table".to_string(),
                result: None,
            }])
            .category(Category::Formats)]
    }

    fn run(
        &mut self,
        _name: &str,
        call: &EvaluatedCall,
        input: &NuValue,
    ) -> Result<NuValue, LabeledError> {
        match input {
            NuValue::String { val, .. } => {
                let plist = plist::from_bytes(val.as_bytes())
                    .map_err(|e| build_label_error(format!("{}", e), &call.head))?;
                let converted = convert_plist_value(&plist)?;
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

fn convert_plist_value(plist_val: &PlistValue) -> Result<NuValue, LabeledError> {
    let span = Span::test_data();
    match plist_val {
        PlistValue::String(s) => Ok(NuValue::string(s.to_owned(), span)),
        PlistValue::Boolean(b) => Ok(NuValue::bool(*b, span)),
        PlistValue::Real(r) => Ok(NuValue::float(*r, span)),
        PlistValue::Date(d) => Ok(NuValue::date(convert_date(d), span)),
        PlistValue::Integer(i) => {
            let signed = i
                .as_signed()
                .ok_or_else(|| build_label_error(format!("Cannot convert {i} to i64"), &span))?;
            Ok(NuValue::int(signed, span))
        }
        PlistValue::Uid(uid) => Ok(NuValue::float(uid.get() as f64, span)),
        PlistValue::Data(data) => Ok(NuValue::binary(data.to_owned(), span)),
        PlistValue::Array(arr) => Ok(NuValue::list(convert_array(arr)?, span)),
        PlistValue::Dictionary(dict) => Ok(convert_dict(dict)?),
        _ => Ok(NuValue::nothing(span)),
    }
}

fn convert_dict(dict: &Dictionary) -> Result<NuValue, LabeledError> {
    let cols: Vec<String> = dict.keys().cloned().collect();
    let vals: Result<Vec<NuValue>, LabeledError> = dict.values().map(convert_plist_value).collect();
    Ok(NuValue::record(
        Record::from_raw_cols_vals(cols, vals?),
        Span::test_data(),
    ))
}

fn convert_array(plist_array: &[PlistValue]) -> Result<Vec<NuValue>, LabeledError> {
    plist_array.iter().map(convert_plist_value).collect()
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
    use plist::Uid;
    use std::time::SystemTime;

    #[test]
    fn test_convert_string() {
        let plist_val = PlistValue::String("hello".to_owned());
        let result = convert_plist_value(&plist_val);
        assert_eq!(
            result,
            Ok(NuValue::string("hello".to_owned(), Span::test_data()))
        );
    }

    #[test]
    fn test_convert_boolean() {
        let plist_val = PlistValue::Boolean(true);
        let result = convert_plist_value(&plist_val);
        assert_eq!(result, Ok(NuValue::bool(true, Span::test_data())));
    }

    #[test]
    fn test_convert_real() {
        let plist_val = PlistValue::Real(3.14);
        let result = convert_plist_value(&plist_val);
        assert_eq!(result, Ok(NuValue::float(3.14, Span::test_data())));
    }

    #[test]
    fn test_convert_integer() {
        let plist_val = PlistValue::Integer(42.into());
        let result = convert_plist_value(&plist_val);
        assert_eq!(result, Ok(NuValue::int(42, Span::test_data())));
    }

    #[test]
    fn test_convert_uid() {
        let v = 12345678_u64;
        let uid = Uid::new(v);
        let plist_val = PlistValue::Uid(uid);
        let result = convert_plist_value(&plist_val);
        assert_eq!(result, Ok(NuValue::float(v as f64, Span::test_data())));
    }

    #[test]
    fn test_convert_data() {
        let data = vec![0x41, 0x42, 0x43];
        let plist_val = PlistValue::Data(data.clone());
        let result = convert_plist_value(&plist_val);
        assert_eq!(result, Ok(NuValue::binary(data, Span::test_data())));
    }

    #[test]
    fn test_convert_date() {
        let epoch = SystemTime::UNIX_EPOCH;
        let plist_date = epoch.into();

        let datetime = convert_date(&plist_date);
        assert_eq!(1970, datetime.year());
        assert_eq!(1, datetime.month());
        assert_eq!(1, datetime.day());
    }

    #[test]
    fn test_convert_dict() {
        let mut dict = Dictionary::new();
        dict.insert("a".to_string(), PlistValue::String("c".to_string()));
        dict.insert("b".to_string(), PlistValue::String("d".to_string()));
        let nu_dict = convert_dict(&dict).unwrap();
        assert_eq!(
            nu_dict,
            NuValue::record(
                Record::from_raw_cols_vals(
                    vec!["a".to_string(), "b".to_string()],
                    vec![
                        NuValue::string("c".to_string(), Span::test_data()),
                        NuValue::string("d".to_string(), Span::test_data())
                    ]
                ),
                Span::test_data()
            )
        );
    }

    #[test]
    fn test_convert_array() {
        let mut arr = Vec::new();
        arr.push(PlistValue::String("a".to_string()));
        arr.push(PlistValue::String("b".to_string()));
        let nu_arr = convert_array(&arr).unwrap();
        assert_eq!(
            nu_arr,
            vec![
                NuValue::string("a".to_string(), Span::test_data()),
                NuValue::string("b".to_string(), Span::test_data())
            ]
        );
    }
}
