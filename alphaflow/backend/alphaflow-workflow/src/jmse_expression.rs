use alphaflow_jmes::{
    functions::{ArgumentType, CustomFunction, Signature},
    Arcvar, Expression, Runtime, Variable,
};
use once_cell::sync::Lazy;
use serde_json::{json, Value};

// =============== 全局: 自定义运行时，包含内置 + 自定义函数 ===============
static CUSTOM_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    // 如果启用了 `sync` feature，那么 `Arcvar` = `Arc<Variable>`，可跨线程安全共享
    let mut rt = Runtime::new();
    // 1) 注册 JMESPath 内置函数
    rt.register_builtin_functions();

    // 2) 注册一个自定义函数 uppercase(s) => 把字符串 s 转为大写
    //    签名: 只能接受单个 String
    rt.register_function(
        "uppercase",
        Box::new(CustomFunction::new(
            Signature::new(vec![ArgumentType::String], None),
            Box::new(|args: &[Arcvar], _ctx| {
                let s = args[0].as_string().cloned().unwrap_or_default();
                Ok(Arcvar::new(Variable::String(s.to_ascii_uppercase())))
            }),
        )),
    );

    // 3) 注册一个自定义函数 split(delim, input) => array of string
    rt.register_function(
        "split",
        Box::new(CustomFunction::new(
            // 签名: split(delim: string, input: string)
            Signature::new(vec![ArgumentType::String, ArgumentType::String], None),
            Box::new(|args: &[Arcvar], _ctx| {
                // 从 Option<&String> 转为 String，若无值则使用 "" 做默认
                let delim = args[0].as_string().cloned().unwrap_or_default();
                let input = args[1].as_string().cloned().unwrap_or_default();

                // 按分隔符拆分，并将每段包装为 Arcvar(Variable::String)
                let splitted: Vec<Arcvar> = input
                    .split(&delim)
                    .map(|part| Arcvar::new(Variable::String(part.to_string())))
                    .collect();

                // 返回一个 Array(Arcvar)
                Ok(Arcvar::new(Variable::Array(splitted)))
            }),
        )),
    );

    rt
});

// =============== 帮助函数: 编译+执行表达式 & 得到 serde_json::Value ===============
fn run_expr(expr_str: &str, data: &Value) -> Value {
    let expr: Expression = match CUSTOM_RUNTIME.compile(expr_str) {
        Ok(e) => e,
        Err(e) => panic!("Compile error on '{}': {}", expr_str, e),
    };
    let var = expr.search(data).expect("Search error");
    serde_json::to_value(&var).expect("Serialization error")
}

// =============== 各种场景测试 ===============
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use super::*;
    use env_logger;
    use log::{debug, trace};
    use std::sync::Once;

    // 使用 Once 确保日志只初始化一次
    static INIT: Once = Once::new();

    fn init_logger() {
        INIT.call_once(|| {
            env_logger::init();
        });
    }

    // (1) 基础字段访问: location.country
    #[test]
    fn s01_basic_field_access() {
        let data = json!({"location":{"country": "UK"}});
        let out = run_expr("location.country", &data);
        assert_eq!(out, json!("UK"));
    }

    // (2) 嵌套对象访问: location.capital.name
    #[test]
    fn s02_nested_object_access() {
        let data = json!({"location":{"capital":{"name":"London"}}});
        let out = run_expr("location.capital.name", &data);
        assert_eq!(out, json!("London"));
    }

    // (3) 数组索引: arr[2]
    #[test]
    fn s03_array_index() {
        let data = json!({"arr": ["A","B","C"]});
        let out = run_expr("arr[2]", &data);
        assert_eq!(out, json!("C"));
    }

    // (4) 遍历投影: arr[*].field
    #[test]
    fn s04_projection() {
        let data = json!({
            "arr":[ {"field":10}, {"field":20}, {"field":30} ]
        });
        let out = run_expr("arr[*].field", &data);
        assert_eq!(out, json!([10, 20, 30]));
    }

    // (5) 条件过滤: events[? event == 'login']
    #[test]
    fn s05_filter() {
        let data = json!({
            "events":[
                {"event":"login","user":"Alice"},
                {"event":"logout","user":"Bob"},
                {"event":"login","user":"Charlie"}
            ]
        });
        let out = run_expr("events[? event == 'login']", &data);
        assert_eq!(
            out,
            json!([
                {"event":"login","user":"Alice"},
                {"event":"login","user":"Charlie"}
            ])
        );
    }

    // (6) 原本: events[? event=='login'][*].user => interpreter缺陷 => 空[]
    //    改成: map(&(@.user), events[? event=='login']) => ["Alice","Charlie"]
    // #[test]
    // fn s06_filter_projection() {
    //     init_logger();
    //     let data = json!({
    //         "events":[
    //             {"event":"login","user":"Alice"},
    //             {"event":"logout","user":"Bob"},
    //             {"event":"login","user":"Charlie"}
    //         ]
    //     });
    //     let out = run_expr("map(&(@.user), events[? event=='login'])", &data);
    //     assert_eq!(out, json!(["Alice", "Charlie"]));
    // }
    #[test]
    fn s06_filter_projection() {
        init_logger();
        let data = json!({
            "events": [
                {"event": "login", "user": "Alice"},
                {"event": "logout", "user": "Bob"},
                {"event": "login", "user": "Charlie"}
            ]
        });
        // 使用正常的表达式，过滤后取 user 字段
        let out = run_expr("events[? event=='login'][*].user", &data);
        assert_eq!(out, json!(["Alice", "Charlie"]));
    }

    // (7) 多管道操作: arr[*].value | sort(@) | max(@)
    #[test]
    fn s07_pipeline() {
        let data = json!({"arr":[{"value":1},{"value":10},{"value":5}]});
        let out = run_expr("arr[*].value | sort(@) | max(@)", &data);
        assert_eq!(out, json!(10));
    }

    // (8) 内置函数: length(arr)
    #[test]
    fn s08_builtin_function() {
        let data = json!({"arr":[10,20,30]});
        let out = run_expr("length(arr)", &data);
        assert_eq!(out, json!(3));
    }

    // (9) 自定义函数: uppercase(...)
    #[test]
    fn s09_custom_function() {
        let data = json!({"msg":"hello world"});
        let out = run_expr("uppercase(msg)", &data);
        assert_eq!(out, json!("HELLO WORLD"));
    }

    // (10) 复合表达式: 先过滤 => [ "Alice","Charlie" ] => 然后 uppercase
    //    改用 map(&uppercase(@), events[? event=='login'][*].user)
    #[test]
    fn s10_complex_expr() {
        init_logger();
        let data = json!({
            "events":[
                {"event":"login","user":"Alice"},
                {"event":"logout","user":"Bob"},
                {"event":"login","user":"Charlie"}
            ]
        });
        let out = run_expr(
            "map(&uppercase(@), events[? event=='login'][*].user)",
            &data,
        );
        assert_eq!(out, json!(["ALICE", "CHARLIE"]));
    }

    // #[test]
    // fn s10_complex_expr() {
    //     let data = json!({
    //         "events":[
    //             {"event":"login","user":"Alice"},
    //             {"event":"logout","user":"Bob"},
    //             {"event":"login","user":"Charlie"}
    //         ]
    //     });
    //     // 用 map(&uppercase(@.user), events[? event=='login'])
    //     let out = run_expr("map(&uppercase(@.user), events[? event=='login'])", &data);
    //     assert_eq!(out, json!(["ALICE", "CHARLIE"]));
    // }

    // (11) 访问 nodeVal.arr[1]
    #[test]
    fn s11_node_access() {
        let data = json!({"nodeVal":{"arr":["a","b","c"]}});
        let out = run_expr("nodeVal.arr[1]", &data);
        assert_eq!(out, json!("b"));
    }

    // (12) 基本访问: someKey
    #[test]
    fn s12_json_access() {
        let data = json!({"someKey":"value"});
        let out = run_expr("someKey", &data);
        assert_eq!(out, json!("value"));
    }

    // (13) 数组越界 => null
    #[test]
    fn s13_out_of_range() {
        let data = json!({"arr":[10,20]});
        let out = run_expr("arr[999]", &data);
        assert_eq!(out, Value::Null);
    }

    // (14) 字段含特殊字符 => ['field.with.dots']
    #[test]
    fn s14_special_field_name() {
        let data = json!({"field.with.dots":"Got me"});
        let out = run_expr(r#"['field.with.dots']"#, &data);
        assert_eq!(out, json!("Got me"));
    }

    // (15) arr[*].field => array of string => map => uppercase
    #[test]
    fn s15_pipeline_plus_array() {
        let data = json!({"arr":[{"field":"abc"},{"field":"xyz"}]});
        let out = run_expr("map(&uppercase(@), arr[*].field)", &data);
        assert_eq!(out, json!(["ABC", "XYZ"]));
    }

    // (16) 嵌套自定义函数: uppercase( uppercase(msg) )
    #[test]
    fn s16_nested_custom_functions() {
        let data = json!({"msg":"HeLlO"});
        let out = run_expr("uppercase(uppercase(msg))", &data);
        assert_eq!(out, json!("HELLO"));
    }

    // (17) 合并对象: merge(object1, object2)
    #[test]
    fn s17_merge_example() {
        let data = json!({
            "object1": {"a":1},
            "object2": {"b":2}
        });
        let out = run_expr("merge(object1, object2)", &data);
        assert_eq!(out, json!({"a":1,"b":2}));
    }

    // (18) 没有 default 函数 & 不支持 {} 空hash => 注释/忽略
    #[test]
    #[ignore]
    fn s18_default_fn() {
        let data = json!({"foo":null});
        let out = run_expr("default(foo, {})", &data);
        assert_eq!(out, json!({}));
    }

    // (19) split
    //
    // 如果 parse "split(":", "a:b:c")" 有冲突，可以改成单引号: split(':','a:b:c')
    #[test]
    fn s19_split_example() {
        let data = json!({});
        let out = run_expr(r#"split(':','a:b:c')"#, &data);
        assert_eq!(out, json!(["a", "b", "c"]));
    }

    // (20) 嵌套过滤
    #[test]
    fn s20_nested_filter() {
        let data = json!({
            "people": [
                {"name":"Alice","addresses":[{"city":"Berlin"},{"city":"Paris"}]},
                {"name":"Bob","addresses":[{"city":"Rome"}]},
                {"name":"Cathy","addresses":[{"city":"Berlin"},{"city":"Tokyo"}]}
            ]
        });
        let out = run_expr("people[? addresses[? city=='Berlin']]", &data);
        assert_eq!(
            out,
            json!([
                {"name":"Alice","addresses":[{"city":"Berlin"},{"city":"Paris"}]},
                {"name":"Cathy","addresses":[{"city":"Berlin"},{"city":"Tokyo"}]}
            ])
        );
    }
}
