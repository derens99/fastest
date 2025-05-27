use crate::discovery::TestItem;
use crate::error::Result;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Represents a parametrized test case
#[derive(Debug, Clone)]
pub struct ParametrizedTest {
    pub base_test: TestItem,
    pub param_sets: Vec<ParamSet>,
}

/// A set of parameters for a single test invocation
#[derive(Debug, Clone)]
pub struct ParamSet {
    pub id: Option<String>, // Custom ID if provided
    pub values: Vec<Value>, // Parameter values
    pub marks: Vec<String>, // Additional marks for this param set (e.g. "xfail")
    pub is_xfail: bool,     // Whether this specific parameter set is expected to fail
}

/// Parse parametrize decorator and extract parameter information
pub fn parse_parametrize_decorator(decorator: &str) -> Option<(Vec<String>, Vec<ParamSet>)> {
    let decorator = decorator.replace('\n', " ").replace("  ", " ");
    let content = decorator
        .trim_start_matches('@')
        .trim_start_matches("pytest.mark.parametrize")
        .trim_start_matches("fastest.mark.parametrize")
        .trim_start_matches("mark.parametrize")
        .trim_start_matches("parametrize")
        .trim();

    if !content.starts_with('(') || !content.ends_with(')') {
        return None;
    }
    let content_in_paren = content.trim_start_matches('(').trim_end_matches(')');

    let mut quote_count = 0;
    let mut first_comma_pos = None;
    for (i, ch) in content_in_paren.chars().enumerate() {
        match ch {
            '"' | '\'' => quote_count += 1,
            ',' if quote_count % 2 == 0 => {
                first_comma_pos = Some(i);
                break;
            }
            _ => {}
        }
    }
    let first_comma_pos = first_comma_pos?;

    let param_names_str = content_in_paren[..first_comma_pos]
        .trim()
        .trim_matches('"')
        .trim_matches('\'');
    let mut values_and_ids_str = content_in_paren[first_comma_pos + 1..].trim();

    let param_names: Vec<String> = param_names_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if param_names.is_empty() {
        return None;
    }

    let mut overall_ids: Option<Vec<String>> = None;
    if let Some(ids_marker_pos) = values_and_ids_str.rfind(", ids=") {
        let ids_val_str = values_and_ids_str[ids_marker_pos + 6..].trim();
        overall_ids = extract_ids_from_list_str(ids_val_str);
        values_and_ids_str = &values_and_ids_str[..ids_marker_pos];
    } else if let Some(ids_marker_pos) = values_and_ids_str.rfind(",ids=") {
        // no space
        let ids_val_str = values_and_ids_str[ids_marker_pos + 5..].trim();
        overall_ids = extract_ids_from_list_str(ids_val_str);
        values_and_ids_str = &values_and_ids_str[..ids_marker_pos];
    }

    let values_list_str = values_and_ids_str.trim();
    if !values_list_str.starts_with('[') || !values_list_str.ends_with(']') {
        return None;
    }
    let values_list_content = &values_list_str[1..values_list_str.len() - 1];

    let item_strings = split_param_list_into_item_strings(values_list_content)?;

    let mut parsed_param_sets = Vec::new();
    for (idx, item_str) in item_strings.iter().enumerate() {
        let mut param_set = parse_one_parameter_set(item_str, &param_names)?;
        // Apply overall_id if this param_set doesn't have a specific one from pytest.param(id=...)
        if param_set.id.is_none() {
            if let Some(ref ids) = overall_ids {
                if let Some(id_val) = ids.get(idx) {
                    param_set.id = Some(id_val.clone());
                }
            }
        }
        parsed_param_sets.push(param_set);
    }

    Some((param_names, parsed_param_sets))
}

fn split_param_list_into_item_strings(list_content_str: &str) -> Option<Vec<String>> {
    let mut items = Vec::new();
    let mut current_item_str = String::new();
    let mut paren_depth = 0;
    let mut bracket_depth = 0;
    let mut in_string = false;
    let mut string_char = ' ';

    for ch in list_content_str.chars() {
        match ch {
            '\'' | '"' => {
                if in_string && ch == string_char {
                    in_string = false;
                } else if !in_string {
                    in_string = true;
                    string_char = ch;
                }
                current_item_str.push(ch);
            }
            '(' if !in_string => {
                paren_depth += 1;
                current_item_str.push(ch);
            }
            ')' if !in_string => {
                paren_depth -= 1;
                current_item_str.push(ch);
            }
            '[' if !in_string => {
                bracket_depth += 1;
                current_item_str.push(ch);
            }
            ']' if !in_string => {
                bracket_depth -= 1;
                current_item_str.push(ch);
            }
            ',' if !in_string && paren_depth == 0 && bracket_depth == 0 => {
                let trimmed = current_item_str.trim();
                // Strip inline comments
                let cleaned = if trimmed.contains('#') && !in_string {
                    strip_inline_comment(trimmed)
                } else {
                    trimmed.to_string()
                };
                if !cleaned.is_empty() {
                    items.push(cleaned);
                }
                current_item_str.clear();
            }
            _ => current_item_str.push(ch),
        }
    }

    let final_item = current_item_str.trim();
    if !final_item.is_empty() {
        // Strip inline comments from the last item too
        let cleaned = if final_item.contains('#') {
            strip_inline_comment(final_item)
        } else {
            final_item.to_string()
        };
        if !cleaned.is_empty() {
            items.push(cleaned);
        }
    }
    Some(items)
}

// Helper function to strip inline comments, being careful about strings
fn strip_inline_comment(s: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut string_char = ' ';
    let mut paren_depth = 0;
    let mut bracket_depth = 0;

    for ch in s.chars() {
        match ch {
            '\'' | '"' => {
                if in_string && ch == string_char {
                    in_string = false;
                } else if !in_string {
                    in_string = true;
                    string_char = ch;
                }
                result.push(ch);
            }
            '(' if !in_string => {
                paren_depth += 1;
                result.push(ch);
            }
            ')' if !in_string => {
                paren_depth -= 1;
                result.push(ch);
            }
            '[' if !in_string => {
                bracket_depth += 1;
                result.push(ch);
            }
            ']' if !in_string => {
                bracket_depth -= 1;
                result.push(ch);
            }
            '#' if !in_string && paren_depth == 0 && bracket_depth == 0 => {
                // Found comment marker, stop here
                break;
            }
            _ => result.push(ch),
        }
    }

    result.trim().to_string()
}

fn parse_one_parameter_set(item_str: &str, param_names: &[String]) -> Option<ParamSet> {
    let num_params = param_names.len();
    let trimmed_item = item_str.trim();

    if trimmed_item.starts_with("pytest.param(") && trimmed_item.ends_with(')') {
        let content_in_param_call = &trimmed_item[13..trimmed_item.len() - 1];

        let values_part_str; // Declare here without initializing or mut
        let mut id_str_opt: Option<&str> = None;
        let mut marks_str_opt: Option<&str> = None;

        let mut temp_content = content_in_param_call;
        let mut found_kwarg_in_iteration;

        loop {
            found_kwarg_in_iteration = false;

            // First, check which kwarg comes last (rightmost) in temp_content
            let id_pos = temp_content.rfind(", id=");
            let marks_pos = temp_content.rfind(", marks=");

            // Process the rightmost kwarg first
            match (id_pos, marks_pos) {
                (Some(id_p), Some(marks_p)) => {
                    if marks_p > id_p {
                        // marks= is rightmost, process it first
                        let potential_marks_val_part = &temp_content[marks_p + 8..];
                        let (val_str, next_temp_content) = extract_kwarg_value_and_remaining(
                            potential_marks_val_part,
                            temp_content,
                            marks_p,
                        );
                        marks_str_opt = Some(val_str);
                        temp_content = next_temp_content;
                        found_kwarg_in_iteration = true;
                    } else {
                        // id= is rightmost, process it first
                        let potential_id_val_part = &temp_content[id_p + 5..];
                        let (val_str, next_temp_content) = extract_kwarg_value_and_remaining(
                            potential_id_val_part,
                            temp_content,
                            id_p,
                        );
                        id_str_opt = Some(val_str);
                        temp_content = next_temp_content;
                        found_kwarg_in_iteration = true;
                    }
                }
                (Some(id_p), None) => {
                    // Only id= found
                    let potential_id_val_part = &temp_content[id_p + 5..];
                    let (val_str, next_temp_content) = extract_kwarg_value_and_remaining(
                        potential_id_val_part,
                        temp_content,
                        id_p,
                    );
                    id_str_opt = Some(val_str);
                    temp_content = next_temp_content;
                    found_kwarg_in_iteration = true;
                }
                (None, Some(marks_p)) => {
                    // Only marks= found
                    let potential_marks_val_part = &temp_content[marks_p + 8..];
                    let (val_str, next_temp_content) = extract_kwarg_value_and_remaining(
                        potential_marks_val_part,
                        temp_content,
                        marks_p,
                    );
                    marks_str_opt = Some(val_str);
                    temp_content = next_temp_content;
                    found_kwarg_in_iteration = true;
                }
                (None, None) => {
                    // No more kwargs found
                }
            }

            if !found_kwarg_in_iteration {
                values_part_str = temp_content.trim(); // Assign the rest to values_part_str
                break;
            }
        }

        let id = id_str_opt.and_then(extract_pytest_param_id);
        let (marks, is_xfail) =
            marks_str_opt.map_or((Vec::new(), false), extract_pytest_param_marks);

        let parsed_values = if num_params == 1 {
            // If there's only one param name, the entire values_part_str is that single parameter.
            vec![parse_single_value(values_part_str)?]
        } else {
            // Multiple param names, expect values_part_str to be a tuple like "(val1, val2)"
            let vp_trimmed = values_part_str.trim();
            if vp_trimmed.starts_with('(') && vp_trimmed.ends_with(')') {
                parse_comma_separated_values(&vp_trimmed[1..vp_trimmed.len() - 1])?
            } else {
                // If not wrapped in parentheses, parse as comma-separated values directly
                // This handles cases like pytest.param(1, "a", id="...", marks=...)
                parse_comma_separated_values(vp_trimmed)?
            }
        };
        if parsed_values.len() != num_params {
            return None;
        }
        Some(ParamSet {
            id,
            values: parsed_values,
            marks,
            is_xfail,
        })
    } else {
        // Regular item (not pytest.param)
        let parsed_values = if num_params == 1 {
            vec![parse_single_value(trimmed_item)?]
        } else {
            if trimmed_item.starts_with('(') && trimmed_item.ends_with(')') {
                parse_comma_separated_values(&trimmed_item[1..trimmed_item.len() - 1])?
            } else {
                return None;
            }
        };
        if parsed_values.len() != num_params {
            return None;
        }
        Some(ParamSet {
            id: None,
            values: parsed_values,
            marks: Vec::new(),
            is_xfail: false,
        })
    }
}

// Helper for parsing pytest.param kwargs to avoid consuming part of another kwarg or value
fn extract_kwarg_value_and_remaining<'a>(
    value_part_candidate: &'a str,
    original_before_kw: &'a str,
    kw_marker_pos: usize,
) -> (&'a str, &'a str) {
    // (kwarg_value, content_before_this_kwarg)
    let mut balance = 0;
    let mut end_pos = value_part_candidate.len();
    let mut in_str = false;
    let mut str_char = ' ';

    for (i, char_code) in value_part_candidate.char_indices() {
        match char_code {
            '\'' | '"' => {
                if in_str && char_code == str_char {
                    in_str = false;
                } else if !in_str {
                    in_str = true;
                    str_char = char_code;
                }
            }
            '(' | '[' | '{' if !in_str => balance += 1,
            ')' | ']' | '}' if !in_str => balance -= 1,
            ',' if !in_str && balance == 0 => {
                end_pos = i;
                break;
            }
            _ => {}
        }
    }
    let kwarg_value = value_part_candidate[..end_pos].trim();
    let remaining_before_kw = original_before_kw[..kw_marker_pos].trim();
    (kwarg_value, remaining_before_kw)
}

fn extract_pytest_param_id(id_str: &str) -> Option<String> {
    if id_str.is_empty() {
        return None;
    }
    Some(id_str.trim_matches('"').trim_matches('\'').to_string())
}

fn extract_pytest_param_marks(marks_str: &str) -> (Vec<String>, bool) {
    let mut marks = Vec::new();
    let mut is_xfail = false;
    if marks_str.is_empty() {
        return (marks, is_xfail);
    }

    // Check if marks are in array format [mark1, mark2] or single mark
    let content = if marks_str.starts_with('[') && marks_str.ends_with(']') {
        // Array format: [pytest.mark.xfail, pytest.mark.slow]
        marks_str
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim()
    } else {
        // Single mark: pytest.mark.xfail
        marks_str.trim()
    };

    for mark_item_str in content.split(',') {
        let mark_item = mark_item_str.trim();
        // This is a very simplified check for xfail.
        // A robust version would parse attribute access like pytest.mark.xfail
        if mark_item.ends_with("xfail") {
            is_xfail = true;
            marks.push("xfail".to_string());
        }
    }
    (marks, is_xfail)
}

// Renamed from parse_tuple_values for clarity
fn parse_comma_separated_values(cs_str: &str) -> Option<Vec<Value>> {
    let mut values = Vec::new();
    let mut current_value_str = String::new();
    let mut paren_depth = 0;
    let mut bracket_depth = 0;
    let mut in_string = false;
    let mut string_char = ' ';

    for ch in cs_str.chars() {
        match ch {
            '\'' | '"' => {
                if in_string && ch == string_char {
                    in_string = false;
                } else if !in_string {
                    in_string = true;
                    string_char = ch;
                }
                current_value_str.push(ch);
            }
            '(' if !in_string => {
                paren_depth += 1;
                current_value_str.push(ch);
            }
            ')' if !in_string => {
                paren_depth -= 1;
                current_value_str.push(ch);
            }
            '[' if !in_string => {
                bracket_depth += 1;
                current_value_str.push(ch);
            }
            ']' if !in_string => {
                bracket_depth -= 1;
                current_value_str.push(ch);
            }
            ',' if !in_string && paren_depth == 0 && bracket_depth == 0 => {
                // Process the current value, stripping any inline comment
                let part_to_parse = current_value_str.trim();
                let cleaned_part = if !in_string && part_to_parse.contains('#') {
                    // Find the comment start, but only if we're not inside nested structures
                    let mut comment_pos = None;
                    let mut temp_paren = 0;
                    let mut temp_bracket = 0;
                    let mut temp_in_str = false;
                    let mut temp_str_char = ' ';

                    for (i, c) in part_to_parse.char_indices() {
                        match c {
                            '\'' | '"' => {
                                if temp_in_str && c == temp_str_char {
                                    temp_in_str = false;
                                } else if !temp_in_str {
                                    temp_in_str = true;
                                    temp_str_char = c;
                                }
                            }
                            '(' if !temp_in_str => temp_paren += 1,
                            ')' if !temp_in_str => temp_paren -= 1,
                            '[' if !temp_in_str => temp_bracket += 1,
                            ']' if !temp_in_str => temp_bracket -= 1,
                            '#' if !temp_in_str && temp_paren == 0 && temp_bracket == 0 => {
                                comment_pos = Some(i);
                                break;
                            }
                            _ => {}
                        }
                    }

                    if let Some(pos) = comment_pos {
                        part_to_parse[..pos].trim()
                    } else {
                        part_to_parse
                    }
                } else {
                    part_to_parse
                };

                if !cleaned_part.is_empty() {
                    if let Some(val) = parse_single_value(cleaned_part) {
                        values.push(val);
                    } else {
                        return None;
                    }
                }
                current_value_str.clear();
            }
            _ => current_value_str.push(ch),
        }
    }

    // Process the last value
    let last_part_to_parse = current_value_str.trim();
    if !last_part_to_parse.is_empty() {
        // Strip potential trailing comment from the very last part
        let cleaned_part = if !in_string && last_part_to_parse.contains('#') {
            let mut comment_pos = None;
            let mut temp_paren = 0;
            let mut temp_bracket = 0;
            let mut temp_in_str = false;
            let mut temp_str_char = ' ';

            for (i, c) in last_part_to_parse.char_indices() {
                match c {
                    '\'' | '"' => {
                        if temp_in_str && c == temp_str_char {
                            temp_in_str = false;
                        } else if !temp_in_str {
                            temp_in_str = true;
                            temp_str_char = c;
                        }
                    }
                    '(' if !temp_in_str => temp_paren += 1,
                    ')' if !temp_in_str => temp_paren -= 1,
                    '[' if !temp_in_str => temp_bracket += 1,
                    ']' if !temp_in_str => temp_bracket -= 1,
                    '#' if !temp_in_str && temp_paren == 0 && temp_bracket == 0 => {
                        comment_pos = Some(i);
                        break;
                    }
                    _ => {}
                }
            }

            if let Some(pos) = comment_pos {
                last_part_to_parse[..pos].trim()
            } else {
                last_part_to_parse
            }
        } else {
            last_part_to_parse
        };

        if !cleaned_part.is_empty() {
            if let Some(val) = parse_single_value(cleaned_part) {
                values.push(val);
            } else {
                return None;
            }
        }
    }

    Some(values)
}

fn parse_single_value(value_str: &str) -> Option<Value> {
    let trimmed = value_str.trim();
    if trimmed.is_empty() {
        return None;
    }

    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        let inner = &trimmed[1..trimmed.len() - 1];
        return Some(Value::String(
            inner.to_string().replace("\\'", "'").replace("\\\"", "\""),
        ));
    }
    if trimmed == "True" {
        return Some(Value::Bool(true));
    }
    if trimmed == "False" {
        return Some(Value::Bool(false));
    }
    if trimmed == "None" {
        return Some(Value::Null);
    }

    if (trimmed.starts_with('(') && trimmed.ends_with(')'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        let inner_content = &trimmed[1..trimmed.len() - 1];
        if inner_content.trim().is_empty() {
            // Handles empty tuple () or empty list []
            return Some(Value::Array(Vec::new()));
        }
        let elements = parse_comma_separated_values(inner_content)?;
        return Some(Value::Array(elements));
    }

    if let Ok(num) = trimmed.parse::<i64>() {
        return Some(Value::Number(serde_json::Number::from(num)));
    }
    if let Ok(num) = trimmed.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(num) {
            return Some(Value::Number(n));
        }
        // If from_f64 fails (e.g. NaN, Infinity), it might be better to return None or a specific error/string
    }

    // Stricter: if it's not a recognized literal, it's a parsing failure for parametrize.
    None
}

fn extract_ids_from_list_str(ids_list_str: &str) -> Option<Vec<String>> {
    let trimmed = ids_list_str.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
    }
    let content = &trimmed[1..trimmed.len() - 1];
    if content.trim().is_empty() {
        return Some(Vec::new());
    }
    Some(
        content
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .collect(),
    )
}

/// Expand a test function with parametrize decorators into multiple test items
pub fn expand_parametrized_tests(test: &TestItem, decorators: &[String]) -> Result<Vec<TestItem>> {
    let mut expanded_tests = Vec::new();
    let mut param_info_list: Vec<(Vec<String>, Vec<ParamSet>, Option<Vec<String>>)> = Vec::new();

    for decorator in decorators {
        if decorator.contains("parametrize") {
            if let Some((names, param_sets)) = parse_parametrize_decorator(decorator) {
                // Overall IDs for the *whole* @parametrize decorator are extracted by parse_parametrize_decorator now implicitly
                // because it processes ", ids=" before splitting values. The overall_ids in the tuple below
                // refers to those extracted from the main decorator line.
                // We pass None for now as parse_parametrize_decorator doesn't explicitly return the overall_ids separately from values string processing
                // This part needs to align with how parse_parametrize_decorator structures its output if overall_ids are needed here distinctly
                // For now, individual ParamSet.id (from pytest.param or overall ids applied during its creation) is primary.
                param_info_list.push((
                    names, param_sets,
                    None, /* TODO: Revisit overall_ids if needed distinctly here */
                ));
            }
        }
    }

    if param_info_list.is_empty() {
        return Ok(vec![test.clone()]);
    }

    let test_cases = generate_test_cases_from_param_sets(&param_info_list);

    for case in test_cases.iter() {
        // idx here is overall index across all combined params
        let mut expanded_test = test.clone();

        let final_id_str: String = case.id_override.clone().unwrap_or_else(|| {
            // If no id from pytest.param() or from overall ids=[], generate one.
            // The overall_ids from the decorator are implicitly handled by parse_parametrize_decorator assigning to param_set.id
            // if param_set.id was None. So case.id_override should be correctly populated.
            format_param_id(&case.params)
        });

        expanded_test.id = format!("{}[{}]", test.id, final_id_str);
        expanded_test.name = format!("{}[{}]", test.function_name, final_id_str);
        expanded_test.is_xfail = case.is_xfail;

        let params_json_str = if case.params.len() == 1
            && case.params.keys().next().map_or(false, |k| k == "coords")
        {
            let single_arg_value = case.params.values().next().unwrap_or(&Value::Null).clone();
            serde_json::to_string(&json!([single_arg_value])).unwrap_or_default()
        } else {
            serde_json::to_string(&case.params).unwrap_or_default()
        };

        expanded_test
            .decorators
            .push(format!("__params__={}", params_json_str));
        if case.is_xfail {
            expanded_test.decorators.push("__xfail__=True".to_string());
        }
        expanded_tests.push(expanded_test);
    }
    Ok(expanded_tests)
}

#[derive(Debug)]
struct TestCase {
    params: HashMap<String, Value>,
    id_override: Option<String>,
    is_xfail: bool,
}

fn generate_test_cases_from_param_sets(
    param_info_list: &[(Vec<String>, Vec<ParamSet>, Option<Vec<String>>)], // Option<Vec<String>> is for overall decorator ids
) -> Vec<TestCase> {
    if param_info_list.is_empty() {
        return vec![];
    }

    let (first_param_names, first_actual_param_sets, _first_overall_ids_decorator_level) =
        &param_info_list[0];
    let mut accumulated_cases: Vec<TestCase> = first_actual_param_sets
        .iter()
        .map(|p_set| {
            let mut params_map = HashMap::new();
            for (name_idx, name) in first_param_names.iter().enumerate() {
                if let Some(value) = p_set.values.get(name_idx) {
                    params_map.insert(name.clone(), value.clone());
                } else { /* Param count mismatch with values, error or skip */
                }
            }
            TestCase {
                params: params_map,
                id_override: p_set.id.clone(),
                is_xfail: p_set.is_xfail,
            }
        })
        .collect();

    for (current_param_names, current_actual_param_sets, _current_overall_ids_decorator_level) in
        param_info_list.iter().skip(1)
    {
        let mut next_acc_cases = Vec::new();
        for existing_case in &accumulated_cases {
            for p_set_for_current_decorator in current_actual_param_sets {
                let mut new_params_map = existing_case.params.clone();
                for (name_idx, name) in current_param_names.iter().enumerate() {
                    if let Some(value) = p_set_for_current_decorator.values.get(name_idx) {
                        new_params_map.insert(name.clone(), value.clone());
                    } else { /* Param count mismatch */
                    }
                }
                // ID override: pytest.param takes precedence. If multiple decorators, this simple override might need thought.
                // For now, the ID from the latest pytest.param affecting this combined case, or existing.
                let final_id = p_set_for_current_decorator
                    .id
                    .clone()
                    .or_else(|| existing_case.id_override.clone());
                let combined_xfail = existing_case.is_xfail || p_set_for_current_decorator.is_xfail;
                next_acc_cases.push(TestCase {
                    params: new_params_map,
                    id_override: final_id,
                    is_xfail: combined_xfail,
                });
            }
        }
        accumulated_cases = next_acc_cases;
    }
    accumulated_cases
}

fn format_param_id(params: &HashMap<String, Value>) -> String {
    let mut parts = Vec::new();
    let mut keys: Vec<_> = params.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(value) = params.get(key) {
            let formatted = match value {
                Value::String(s) => s.replace('-', "_"),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => (if *b { "True" } else { "False" }).to_string(),
                Value::Null => "None".to_string(),
                Value::Array(arr) => format!(
                    "[{}]",
                    arr.iter()
                        .map(format_param_id_val)
                        .collect::<Vec<_>>()
                        .join(",")
                )
                .replace("-", "_"),
                Value::Object(_) => "{object}".to_string(),
            };
            parts.push(formatted);
        }
    }
    parts.join("-")
}

fn format_param_id_val(val: &Value) -> String {
    match val {
        Value::String(s) => s.replace('-', "_"),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => (if *b { "True" } else { "False" }).to_string(),
        Value::Null => "None".to_string(),
        Value::Array(arr) => format!(
            "[{}]",
            arr.iter()
                .map(format_param_id_val)
                .collect::<Vec<_>>()
                .join(",")
        )
        .replace("-", "_"),
        Value::Object(_) => "{object}".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_parametrize() {
        let decorator = r#"@pytest.mark.parametrize("x", [1, 2, 3])"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());
        let (names, param_sets) = result.unwrap();
        assert_eq!(names, vec!["x"]);
        assert_eq!(param_sets.len(), 3);
        assert_eq!(param_sets[0].values, vec![Value::Number(1.into())]);
    }

    #[test]
    fn test_parse_tuple_parametrize() {
        let decorator = r#"@pytest.mark.parametrize("x,y,expected", [(1,2,3), (4,5,9)])"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());
        let (names, param_sets) = result.unwrap();
        assert_eq!(names, vec!["x", "y", "expected"]);
        assert_eq!(param_sets.len(), 2);
        assert_eq!(
            param_sets[0].values,
            vec![
                Value::Number(1.into()),
                Value::Number(2.into()),
                Value::Number(3.into())
            ]
        );
    }

    #[test]
    fn test_parse_single_value_simple_tuple() {
        let result = parse_single_value("(1, 2, 'hello')");
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            Value::Array(vec![
                Value::Number(1.into()),
                Value::Number(2.into()),
                Value::String("hello".to_string())
            ])
        );
    }

    #[test]
    fn test_parse_single_value_nested_tuple() {
        let result = parse_single_value("((0,0), (3,4), 5.0)");
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            Value::Array(vec![
                Value::Array(vec![Value::Number(0.into()), Value::Number(0.into())]),
                Value::Array(vec![Value::Number(3.into()), Value::Number(4.into())]),
                Value::Number(serde_json::Number::from_f64(5.0).unwrap())
            ])
        );
    }

    #[test]
    fn test_parse_coords_param() {
        let decorator =
            r#"@pytest.mark.parametrize("coords", [((0,0), (3,4), 5.0), ((1,1), (2,2), 1.41)])"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());
        let (names, param_sets) = result.unwrap();
        assert_eq!(names, vec!["coords"]);
        assert_eq!(param_sets.len(), 2);
        // Check first param set for "coords"
        assert_eq!(param_sets[0].values.len(), 1); // "coords" is one parameter
        assert_eq!(
            param_sets[0].values[0],
            Value::Array(vec![
                Value::Array(vec![Value::Number(0.into()), Value::Number(0.into())]),
                Value::Array(vec![Value::Number(3.into()), Value::Number(4.into())]),
                Value::Number(serde_json::Number::from_f64(5.0).unwrap())
            ])
        );
    }

    #[test]
    fn test_parse_pytest_param_with_id_and_xfail_mark() {
        let decorator = r#"@pytest.mark.parametrize("arg1, arg2", [pytest.param(1, "a", id="myid", marks=pytest.mark.xfail)])"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());
        let (names, param_sets) = result.unwrap();
        assert_eq!(names, vec!["arg1", "arg2"]);
        assert_eq!(param_sets.len(), 1);
        let pset = &param_sets[0];
        assert_eq!(
            pset.values,
            vec![Value::Number(1.into()), Value::String("a".to_string())]
        );
        assert_eq!(pset.id, Some("myid".to_string()));
        assert!(pset.is_xfail);
        assert!(pset.marks.contains(&"xfail".to_string()));
    }
    #[test]
    fn test_parse_pytest_param_with_multiple_marks_and_id() {
        let decorator = r#"@pytest.mark.parametrize("value", [pytest.param(0, marks=[pytest.mark.foo, pytest.mark.xfail], id="zero_case")])"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());
        let (names, param_sets) = result.unwrap();
        assert_eq!(names, vec!["value"]);
        assert_eq!(param_sets.len(), 1);
        let pset = &param_sets[0];
        assert_eq!(pset.values, vec![Value::Number(0.into())]);
        assert_eq!(pset.id, Some("zero_case".to_string()));
        assert!(pset.is_xfail);
        assert!(pset.marks.contains(&"xfail".to_string()));
    }

    #[test]
    fn test_parse_decorator_with_overall_ids() {
        let decorator =
            r#"@pytest.mark.parametrize("test_input,expected", [(1,1),(2,4)], ids=["one", "two"])"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());
        let (names, param_sets) = result.unwrap();
        assert_eq!(names, vec!["test_input", "expected"]);
        assert_eq!(param_sets.len(), 2);
        assert_eq!(param_sets[0].id, Some("one".to_string()));
        assert_eq!(param_sets[1].id, Some("two".to_string()));
    }

    #[test]
    fn test_parse_empty_param_list() {
        let decorator = r#"@pytest.mark.parametrize("x", [])"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());
        let (names, param_sets) = result.unwrap();
        assert_eq!(names, vec!["x"]);
        assert_eq!(param_sets.len(), 0);
    }
}
