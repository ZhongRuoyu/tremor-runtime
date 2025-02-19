// Copyright 2022, The Tremor Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::PathBuf;

use criterion::Throughput;
use criterion::{criterion_group, criterion_main, Bencher, BenchmarkId, Criterion};
use simd_json::ValueAccess;
use tremor_script::ast::ImutExpr;
use tremor_script::interpreter::{Env, LocalStack};

use tremor_script::module::Manager;
use tremor_script::prelude::ExecOpts;
use tremor_value::Value;

fn do_array_flatten<'script>(
    bencher: &mut Bencher,
    invoke_event: &(ImutExpr<'script>, Value<'script>),
) {
    let (invoke, event) = invoke_event;
    let opts = ExecOpts {
        result_needed: true,
        aggr: tremor_script::AggrType::Tick,
    };
    let env = Env::default();
    let local_stack = LocalStack::default();
    bencher.iter(|| {
        invoke
            .run(
                opts,
                &env,
                event,
                &Value::const_null(),
                &Value::const_null(),
                &local_stack,
            )
            .expect("Expected array::flatten call to work");
    });
}

fn array_flatten(c: &mut Criterion) {
    let registry = tremor_script::registry();

    // add the std lib dir to the PATH
    let mut cargo_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    cargo_dir = cargo_dir.canonicalize().unwrap();
    if !cargo_dir.to_string_lossy().contains("tremor-script") {
        cargo_dir.push("tremor-script");
    }
    cargo_dir.push("lib");
    Manager::add_path(&cargo_dir.to_string_lossy()).unwrap();

    let inputs: Vec<(&'static str, Value)> = vec![
        (
            "deeply nested",
            tremor_value::literal!([
                "1", "2", ["3", ["4"]], [[["5", "6"], ["7", {"foo": "bar"}, ["8", "9", "10", "11", "12", "13"]]]]
            ]),
        ),
        (
            "flat",
            tremor_value::literal!([
                "1", "2", ["3", "4", "5", "6"], ["7", {"foo": "bar"}, "8", "9"], ["10"], ["11", "12"], "13"
            ]),
        ),
        ("one", tremor_value::literal!([["1", "2", "3"]])),
        ("baseline", tremor_value::literal!([])),
    ];
    let mut group = c.benchmark_group("array_flatten");
    for (label, input) in inputs {
        group.throughput(Throughput::Elements(
            input.as_array().map(Vec::len).unwrap_or_default() as u64,
        ));

        let mut script = tremor_script::script::Script::parse(
            "use std::array; array::flatten(event)",
            &registry,
        )
        .expect("Invalid script");
        let first_expr = script.script.exprs.remove(0);
        let invoke = first_expr.as_invoke().expect("No invoke");
        let expr = ImutExpr::Invoke1(invoke.clone());
        let input = (expr, input);

        group.bench_with_input(BenchmarkId::from_parameter(label), &input, do_array_flatten);
    }
    group.finish();
}

criterion_group!(benches, array_flatten);
criterion_main!(benches);
