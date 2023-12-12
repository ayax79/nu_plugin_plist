use std::time::SystemTime;

use chrono::{DateTime, FixedOffset, Offset, Utc};
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, PluginExample, PluginSignature, Span, Value as NuValue, Record};
use plist::{Date as PlistDate, Dictionary, Value as PlistValue, Integer};

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
            .category(Category::Experimental),
            PluginSignature::build("to plist")
            .usage("Convert Nu values into plist")
            .switch("binary", "Output plist in binary format", Some('b'))
            .plugin_examples(vec![PluginExample {
                example: "{ a: 3 } | to plist".to_string(),
                description: "Convert a table into a plist file".to_string(),
                result: None,
            }])
            .category(Category::Experimental)
        ]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &NuValue,
    ) -> Result<NuValue, LabeledError> {
        if name == "from plist" {
            match input {
                NuValue::String { val, .. } => {
                    let plist = plist::from_bytes(val.as_bytes())
                        .map_err(|e| build_label_error(format!("{}", e), &input.span()))?;
                    let converted = convert_plist_value(&plist);
                    Ok(converted)
                }
                NuValue::Binary { val, .. } => {
                    let plist = plist::from_bytes(&val)
                        .map_err(|e| build_label_error(format!("{}", e), &input.span()))?;
                    let converted = convert_plist_value(&plist);
                    Ok(converted)
                }
                _ => Err(build_label_error(
                    format!("Invalid input, must be string not: {:?}", input),
                    &call.head,
                )),
            }
        } else {
            let plist_val = convert_nu_value(input)?;
            let mut out = Vec::new();
            if call.has_flag("binary") {
                plist::to_writer_binary(&mut out, &plist_val).map_err(|e| build_label_error(format!("{}", e), &input.span()))?;
                Ok(NuValue::binary(out, input.span()))
            } else {
                plist::to_writer_xml(&mut out, &plist_val).map_err(|e| build_label_error(format!("{}", e), &input.span()))?;
                Ok(NuValue::string(String::from_utf8(out).map_err(|e| build_label_error(format!("{}", e), &input.span()))?, input.span()))
            }
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

fn convert_plist_value(plist_val: &PlistValue) -> NuValue {
    let span = Span::test_data();
    match plist_val {
        PlistValue::String(s) => NuValue::string(s.to_owned(), span),
        PlistValue::Boolean(b) => NuValue::bool(*b, span),
        PlistValue::Real(r) => NuValue::float(*r, span),
        PlistValue::Date(d) => NuValue::date(convert_date(d), span),
        PlistValue::Integer(i) => NuValue::int(i.as_signed().unwrap(), span),
        PlistValue::Uid(uid) => NuValue::float(f64::from_bits(uid.get()), span),
        PlistValue::Data(data) => NuValue::binary(data.to_owned(), span),
        PlistValue::Array(arr) => NuValue::list(convert_array(arr), span),
        PlistValue::Dictionary(dict) => convert_dict(dict),
        _ => NuValue::nothing(span),
    }
}

fn convert_dict(dict: &Dictionary) -> NuValue {
    let cols: Vec<String> = dict.keys().cloned().collect();
    let vals: Vec<NuValue> = dict.values().map(convert_plist_value).collect();
    NuValue::record(Record { cols, vals }, Span::test_data())
}

fn convert_array(plist_array: &Vec<PlistValue>) -> Vec<NuValue> {
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

fn convert_nu_value(nu_val: &NuValue) -> Result<PlistValue, LabeledError> {
    let span = Span::test_data();
    match nu_val {
        NuValue::String { val, .. } => Ok(PlistValue::String(val.to_owned())),
        NuValue::Bool { val, .. } => Ok(PlistValue::Boolean(*val)),
        NuValue::Float { val, .. } => Ok(PlistValue::Real(*val)),
        NuValue::Int { val, .. } => Ok(PlistValue::Integer(Into::<Integer>::into(*val))),
        NuValue::Binary { val, .. } => Ok(PlistValue::Data(val.to_owned())),
        NuValue::Record { val , .. } => {
            convert_nu_dict(val)
        }
        NuValue::List { vals, .. } => {
            Ok(PlistValue::Array(vals.iter().map(|v| convert_nu_value(v)).collect::<Result<_, _>>()?))
        }
        NuValue::Date { val, .. } => Ok(PlistValue::Date(SystemTime::from(val.to_owned()).into())),
        NuValue::LazyRecord { val, .. } => convert_nu_dict(val.collect()?.as_record().unwrap()),
        _ => Err(build_label_error(format!("{:?} is not convertible", nu_val), &span))
    }
}

fn convert_nu_dict(Record { cols, vals }: &Record) -> Result<PlistValue, LabeledError> {
    Ok(PlistValue::Dictionary(cols.iter().zip(vals.iter()).map(|(k, v)| {
        convert_nu_value(v).map(|v| (k.to_owned(), v))
    }).collect::<Result<_, _>>()?))
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
