use std::time::SystemTime;

use chrono::{DateTime, FixedOffset, Offset, Utc};
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, PluginSignature, Span, Value as NuValue};
use plist::{Date as PlistDate, Dictionary, Value as PlistValue};

pub struct NuPlist;

impl Plugin for NuPlist {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![PluginSignature::build("from plist")
            .usage("cat file.plist | from plist")
            .category(Category::Experimental)]
    }

    fn run(
        &mut self,
        _name: &str,
        call: &EvaluatedCall,
        input: &NuValue,
    ) -> Result<NuValue, LabeledError> {
        match input {
            NuValue::String { val, span } => {
                let plist = plist::from_bytes(val.as_bytes())
                    .map_err(|e| build_label_error(format!("{}", e), span))?;
                let converted = convert_plist_value(&plist);
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

fn convert_plist_value(plist_val: &PlistValue) -> NuValue {
    let span = Span::test_data();
    match plist_val {
        PlistValue::String(s) => NuValue::String {
            val: s.to_owned(),
            span,
        },
        PlistValue::Boolean(b) => NuValue::Bool {
            val: *b,
            span,
        },
        PlistValue::Real(r) => NuValue::Float {
            val: *r,
            span,
        },
        PlistValue::Date(d) => NuValue::Date {
            val: convert_date(d),
            span,
        },
        PlistValue::Integer(i) => NuValue::Int {
            val: i.as_signed().unwrap(),
            span,
        },
        PlistValue::Uid(uid) => NuValue::Float {
            val: f64::from_bits(uid.get()),
            span,
        },
        PlistValue::Data(data) => NuValue::Binary {
            val: data.to_owned(),
            span,
        },
        PlistValue::Array(arr) => NuValue::List {
            vals: convert_array(arr),
            span,
        },
        PlistValue::Dictionary(dict) => convert_dict(dict),
        _ => NuValue::Nothing {
            span,
        },
    }
}

fn convert_dict(dict: &Dictionary) -> NuValue {
    let cols: Vec<String> = dict.keys().cloned().collect();
    let vals: Vec<NuValue> = dict
        .values()
        .map(convert_plist_value)
        .collect();
    NuValue::Record {
        cols,
        vals,
        span: Span::test_data(),
    }
}

fn convert_array(plist_array: &Vec<PlistValue>) -> Vec<NuValue> {
    plist_array
        .iter()
        .map(convert_plist_value)
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
