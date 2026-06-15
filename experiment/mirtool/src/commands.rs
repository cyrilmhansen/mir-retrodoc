use crate::error::CliError;
use crate::io::{detect_format, load_image, FileFormat};
use mircap::image::ModuleImage;
use mircap::{FunctionId, Opcode};
use mirplan::{DataSegmentPlan, LoweredInstruction, LoweredOperand, LoweredProgram};
use mirsem::runner::Runner;
use mirsem::trap::ExecutionTrap;
use serde_json::{json, Value as JsonValue};
use std::collections::BTreeMap;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

fn trap_info(trap: &ExecutionTrap) -> (u32, &'static str) {
    match trap {
        ExecutionTrap::StackOverflow { .. } => (1, "StackOverflow"),
        ExecutionTrap::FuelExhausted { .. } => (2, "FuelExhausted"),
        ExecutionTrap::ExplicitTrap { .. } => (3, "ExplicitTrap"),
        ExecutionTrap::OutOfMemory { .. } => (11, "OutOfMemory"),
        ExecutionTrap::HeapStackCollision { .. } => (12, "HeapStackCollision"),
        ExecutionTrap::OutOfBoundsLoad { .. } => (13, "OutOfBoundsLoad"),
        ExecutionTrap::OutOfBoundsStore { .. } => (14, "OutOfBoundsStore"),
        ExecutionTrap::MisalignedLoad { .. } => (15, "MisalignedLoad"),
        ExecutionTrap::MisalignedStore { .. } => (16, "MisalignedStore"),
        ExecutionTrap::AddressOverflow { .. } => (17, "AddressOverflow"),
        _ => (99, "Unknown"),
    }
}

fn trap_name(code: u32) -> &'static str {
    match code {
        1 => "StackOverflow",
        2 => "FuelExhausted",
        3 => "ExplicitTrap",
        11 => "OutOfMemory",
        12 => "HeapStackCollision",
        13 => "OutOfBoundsLoad",
        14 => "OutOfBoundsStore",
        15 => "MisalignedLoad",
        16 => "MisalignedStore",
        17 => "AddressOverflow",
        _ => "Unknown",
    }
}

pub fn cmd_validate(path: &str, format_opt: Option<&str>) -> Result<(), CliError> {
    let load_res = load_image(path, format_opt);
    let image = match load_res {
        Ok(img) => img,
        Err(err) => {
            println!("ERROR: LoadError: {}", err);
            return Ok(());
        }
    };

    match image.validate() {
        Ok(_) => {
            println!("OK");
            Ok(())
        }
        Err(errors) => {
            if let Some(err) = errors.first() {
                println!("ERROR: {:?}: {}", err.kind, err.message);
            } else {
                println!("ERROR: Unknown: Validation failed with no details");
            }
            Ok(())
        }
    }
}

pub fn cmd_encode(input_path: &str, output_path: &str, force: bool) -> Result<(), CliError> {
    let image = load_image(input_path, Some("text"))?;

    // Safety check for existing file
    let out_path = Path::new(output_path);
    if out_path.exists() && !force {
        return Err(CliError::Generic(format!(
            "Output file '{}' already exists. Use --force to overwrite.",
            output_path
        )));
    }

    let bytes = image.to_capnp_bytes();
    std::fs::write(output_path, bytes)?;
    Ok(())
}

pub fn cmd_decode(input_path: &str, format_opt: Option<&str>) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let text = image_to_text(&image);
    print!("{}", text);
    Ok(())
}

pub fn cmd_run(
    input_path: &str,
    format_opt: Option<&str>,
    entry_name: &str,
    show_trace: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let mut runner = Runner::new(image, mirsem::ExecutionProfile::default())?;

    match runner.run_entry_by_name(entry_name, &[]) {
        Ok(res) => {
            if res.values.is_empty() {
                println!("Result: void");
            } else {
                for val in res.values {
                    match val {
                        mirsem::Value::Void => println!("Result: void"),
                        mirsem::Value::I32(v) => println!("Result: i32 {}", v),
                        mirsem::Value::U32(v) => println!("Result: u32 {}", v),
                        mirsem::Value::Addr32(v) => println!("Result: addr32 {}", v),
                        mirsem::Value::I64(v) => println!("Result: i64 {}", v),
                        mirsem::Value::F32(bits) => {
                            println!("Result: f32 {} bits=0x{bits:08x}", f32::from_bits(bits))
                        }
                        mirsem::Value::F64(bits) => {
                            println!("Result: f64 {} bits=0x{bits:016x}", f64::from_bits(bits))
                        }
                    }
                }
            }
            if show_trace {
                print_trace_summary(&runner.trace_snapshot());
            }
            Ok(())
        }
        Err(mirsem::RunError::Trap(trap)) => {
            let (code, name) = trap_info(&trap);
            println!("Trap: {} {}", code, name);
            if show_trace {
                print_trace_summary(&runner.trace_snapshot());
            }
            Ok(())
        }
        Err(err) => Err(CliError::Run(err)),
    }
}

pub fn cmd_plan(input_path: &str, format_opt: Option<&str>) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    print!("{}", mirplan::format_plan(&plan));
    Ok(())
}

pub fn cmd_analyze(
    input_path: &str,
    format_opt: Option<&str>,
    emit_json: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    if emit_json {
        println!("{}", format_effect_summaries_json(&space));
    } else {
        print!("{}", format_effect_summaries(&space));
    }
    Ok(())
}

pub fn cmd_trace_check(
    input_path: &str,
    format_opt: Option<&str>,
    entry_name: &str,
    emit_json: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let mut runner = Runner::new(image, mirsem::ExecutionProfile::default())?;
    match runner.run_entry_by_name(entry_name, &[]) {
        Ok(_) | Err(mirsem::RunError::Trap(_)) => {
            let snapshot = runner.trace_snapshot();
            if emit_json {
                println!("{}", format_trace_check_json(&space, &snapshot));
            } else {
                print!("{}", format_trace_check(&space, &snapshot));
            }
            Ok(())
        }
        Err(err) => Err(CliError::Run(err)),
    }
}

pub fn cmd_cost(
    input_path: &str,
    format_opt: Option<&str>,
    emit_json: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let lowered = mirplan::lower_compile_plan(&plan);
    let cost = mirplan::summarize_cost(&lowered);
    if emit_json {
        println!("{}", format_cost_summary_json(&cost));
    } else {
        print!("{}", format_cost_summary(&cost));
    }
    Ok(())
}

pub fn cmd_trace_cost(
    input_path: &str,
    format_opt: Option<&str>,
    entry_name: &str,
    emit_json: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let lowered = mirplan::lower_compile_plan(&plan);
    let cost = mirplan::summarize_cost(&lowered);
    let mut runner = Runner::new(image, mirsem::ExecutionProfile::default())?;
    match runner.run_entry_by_name(entry_name, &[]) {
        Ok(_) | Err(mirsem::RunError::Trap(_)) => {
            let snapshot = runner.trace_snapshot();
            let report = build_trace_cost_report(&cost, &snapshot);
            if emit_json {
                println!("{}", format_trace_cost_json(&snapshot, &report));
            } else {
                print!("{}", format_trace_cost(&snapshot, &report));
            }
            Ok(())
        }
        Err(err) => Err(CliError::Run(err)),
    }
}

fn format_cost_summary(cost: &mirplan::ProgramCostSummary) -> String {
    let mut out = String::new();
    out.push_str(&format!("cost module {}\n", cost.module_name));
    out.push_str(&format!("  bounded: {}\n", cost.bounded));
    out.push_str("  totals:\n");
    append_cost_counts(&mut out, &cost.totals, 4);
    for function in &cost.functions {
        out.push_str(&format!(
            "  fn f{}#{} {}\n",
            function.function.0, function.id.0, function.name
        ));
        out.push_str(&format!("    bounded: {}\n", function.bounded));
        out.push_str(&format!("    bound_kind: {}\n", function.bound_kind));
        out.push_str("    counts:\n");
        append_cost_counts(&mut out, &function.counts, 6);
    }
    out
}

fn append_cost_counts(out: &mut String, counts: &mirplan::CostCounts, indent: usize) {
    let pad = " ".repeat(indent);
    out.push_str(&format!("{pad}instructions: {}\n", counts.instructions));
    out.push_str(&format!("{pad}branches: {}\n", counts.branches));
    out.push_str(&format!("{pad}calls: {}\n", counts.calls));
    out.push_str(&format!("{pad}memory_reads: {}\n", counts.memory_reads));
    out.push_str(&format!("{pad}memory_writes: {}\n", counts.memory_writes));
    out.push_str(&format!(
        "{pad}memory_addresses: {}\n",
        counts.memory_addresses
    ));
    out.push_str(&format!("{pad}allocations: {}\n", counts.allocations));
    out.push_str(&format!("{pad}traps: {}\n", counts.traps));
}

fn format_cost_summary_json(cost: &mirplan::ProgramCostSummary) -> String {
    let functions = cost
        .functions
        .iter()
        .map(|function| {
            json!({
                "index": function.function.0,
                "id": function.id.0,
                "name": function.name,
                "bounded": function.bounded,
                "bound_kind": function.bound_kind,
                "counts": cost_counts_json(&function.counts)
            })
        })
        .collect::<Vec<_>>();
    json!({
        "kind": "cost",
        "module": {
            "name": cost.module_name
        },
        "bounded": cost.bounded,
        "totals": cost_counts_json(&cost.totals),
        "functions": functions
    })
    .to_string()
}

fn cost_counts_json(counts: &mirplan::CostCounts) -> JsonValue {
    json!({
        "instructions": counts.instructions,
        "branches": counts.branches,
        "calls": counts.calls,
        "memory_reads": counts.memory_reads,
        "memory_writes": counts.memory_writes,
        "memory_addresses": counts.memory_addresses,
        "allocations": counts.allocations,
        "traps": counts.traps
    })
}

#[derive(Clone, Debug)]
struct TraceCostReport {
    module_name: String,
    bounded: bool,
    totals: TraceCostFunctionComparison,
    functions: Vec<TraceCostFunctionComparison>,
}

#[derive(Clone, Debug)]
struct TraceCostFunctionComparison {
    function: Option<usize>,
    id: Option<FunctionId>,
    name: String,
    bounded: bool,
    bound_kind: &'static str,
    predicted: mirplan::CostCounts,
    observed: mirplan::CostCounts,
}

fn build_trace_cost_report(
    cost: &mirplan::ProgramCostSummary,
    snapshot: &mirsem::TraceSnapshot,
) -> TraceCostReport {
    let trace_by_function = snapshot
        .functions
        .iter()
        .map(|trace| (trace.function, trace))
        .collect::<BTreeMap<_, _>>();
    let functions = cost
        .functions
        .iter()
        .map(|function| {
            let trace = trace_by_function.get(&function.id).copied();
            TraceCostFunctionComparison {
                function: Some(function.function.0),
                id: Some(function.id),
                name: function.name.clone(),
                bounded: function.bounded,
                bound_kind: function.bound_kind,
                predicted: function.counts.clone(),
                observed: trace.map(observed_function_cost_counts).unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();
    TraceCostReport {
        module_name: cost.module_name.clone(),
        bounded: cost.bounded,
        totals: TraceCostFunctionComparison {
            function: None,
            id: None,
            name: "totals".to_string(),
            bounded: cost.bounded,
            bound_kind: if cost.bounded {
                "acyclic-structural"
            } else {
                "cyclic-unknown"
            },
            predicted: cost.totals.clone(),
            observed: observed_total_cost_counts(snapshot),
        },
        functions,
    }
}

fn observed_total_cost_counts(snapshot: &mirsem::TraceSnapshot) -> mirplan::CostCounts {
    mirplan::CostCounts {
        instructions: snapshot.executed_instruction_count,
        branches: snapshot.branch_count,
        calls: snapshot.call_instruction_count,
        memory_reads: snapshot.memory_read_count,
        memory_writes: snapshot.memory_write_count,
        memory_addresses: snapshot.address_instruction_count,
        allocations: snapshot.allocation_count,
        traps: snapshot.trap_count,
    }
}

fn observed_function_cost_counts(trace: &mirsem::FunctionTrace) -> mirplan::CostCounts {
    mirplan::CostCounts {
        instructions: trace.executed_instructions,
        branches: trace.branches,
        calls: trace.call_instructions,
        memory_reads: trace.memory_reads,
        memory_writes: trace.memory_writes,
        memory_addresses: trace.address_instructions,
        allocations: trace.allocations,
        traps: trace.traps,
    }
}

fn format_trace_cost(snapshot: &mirsem::TraceSnapshot, report: &TraceCostReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("trace-cost module {}\n", report.module_name));
    out.push_str(&format!(
        "  outcome: {}\n",
        format_trace_outcome(&snapshot.outcome)
    ));
    out.push_str(&format!("  bounded: {}\n", report.bounded));
    out.push_str("  totals:\n");
    append_cost_comparison(&mut out, &report.totals, 4);
    for function in &report.functions {
        out.push_str(&format!(
            "  fn f{}#{} {}\n",
            function.function.unwrap_or_default(),
            function.id.map(|id| id.0).unwrap_or_default(),
            function.name
        ));
        out.push_str(&format!("    bounded: {}\n", function.bounded));
        out.push_str(&format!("    bound_kind: {}\n", function.bound_kind));
        out.push_str("    comparison:\n");
        append_cost_comparison(&mut out, function, 6);
    }
    out
}

fn append_cost_comparison(
    out: &mut String,
    comparison: &TraceCostFunctionComparison,
    indent: usize,
) {
    let pad = " ".repeat(indent);
    append_cost_comparison_line(
        out,
        &pad,
        "instructions",
        comparison.predicted.instructions,
        comparison.observed.instructions,
        comparison.bounded,
    );
    append_cost_comparison_line(
        out,
        &pad,
        "branches",
        comparison.predicted.branches,
        comparison.observed.branches,
        comparison.bounded,
    );
    append_cost_comparison_line(
        out,
        &pad,
        "calls",
        comparison.predicted.calls,
        comparison.observed.calls,
        comparison.bounded,
    );
    append_cost_comparison_line(
        out,
        &pad,
        "memory_reads",
        comparison.predicted.memory_reads,
        comparison.observed.memory_reads,
        comparison.bounded,
    );
    append_cost_comparison_line(
        out,
        &pad,
        "memory_writes",
        comparison.predicted.memory_writes,
        comparison.observed.memory_writes,
        comparison.bounded,
    );
    append_cost_comparison_line(
        out,
        &pad,
        "memory_addresses",
        comparison.predicted.memory_addresses,
        comparison.observed.memory_addresses,
        comparison.bounded,
    );
    append_cost_comparison_line(
        out,
        &pad,
        "allocations",
        comparison.predicted.allocations,
        comparison.observed.allocations,
        comparison.bounded,
    );
    append_cost_comparison_line(
        out,
        &pad,
        "traps",
        comparison.predicted.traps,
        comparison.observed.traps,
        comparison.bounded,
    );
}

fn append_cost_comparison_line(
    out: &mut String,
    pad: &str,
    name: &str,
    predicted: u64,
    observed: u64,
    bounded: bool,
) {
    out.push_str(&format!(
        "{pad}{name}: predicted={} observed={} status={}\n",
        predicted,
        observed,
        cost_comparison_status(predicted, observed, bounded)
    ));
}

fn format_trace_cost_json(snapshot: &mirsem::TraceSnapshot, report: &TraceCostReport) -> String {
    let functions = report
        .functions
        .iter()
        .map(trace_cost_function_json)
        .collect::<Vec<_>>();
    json!({
        "kind": "trace-cost",
        "module": {
            "name": report.module_name
        },
        "outcome": trace_outcome_json(&snapshot.outcome),
        "bounded": report.bounded,
        "totals": trace_cost_comparison_json(&report.totals),
        "functions": functions
    })
    .to_string()
}

fn trace_cost_function_json(function: &TraceCostFunctionComparison) -> JsonValue {
    json!({
        "index": function.function,
        "id": function.id.map(|id| id.0),
        "name": function.name,
        "bounded": function.bounded,
        "bound_kind": function.bound_kind,
        "comparison": trace_cost_comparison_json(function)
    })
}

fn trace_cost_comparison_json(comparison: &TraceCostFunctionComparison) -> JsonValue {
    json!({
        "instructions": cost_comparison_json(
            comparison.predicted.instructions,
            comparison.observed.instructions,
            comparison.bounded
        ),
        "branches": cost_comparison_json(
            comparison.predicted.branches,
            comparison.observed.branches,
            comparison.bounded
        ),
        "calls": cost_comparison_json(
            comparison.predicted.calls,
            comparison.observed.calls,
            comparison.bounded
        ),
        "memory_reads": cost_comparison_json(
            comparison.predicted.memory_reads,
            comparison.observed.memory_reads,
            comparison.bounded
        ),
        "memory_writes": cost_comparison_json(
            comparison.predicted.memory_writes,
            comparison.observed.memory_writes,
            comparison.bounded
        ),
        "memory_addresses": cost_comparison_json(
            comparison.predicted.memory_addresses,
            comparison.observed.memory_addresses,
            comparison.bounded
        ),
        "allocations": cost_comparison_json(
            comparison.predicted.allocations,
            comparison.observed.allocations,
            comparison.bounded
        ),
        "traps": cost_comparison_json(
            comparison.predicted.traps,
            comparison.observed.traps,
            comparison.bounded
        )
    })
}

fn cost_comparison_json(predicted: u64, observed: u64, bounded: bool) -> JsonValue {
    json!({
        "predicted": predicted,
        "observed": observed,
        "status": cost_comparison_status(predicted, observed, bounded)
    })
}

fn cost_comparison_status(predicted: u64, observed: u64, bounded: bool) -> &'static str {
    if !bounded {
        "observed-only"
    } else if predicted == observed {
        "exact"
    } else if observed <= predicted {
        "within-structural-bound"
    } else {
        "exceeds-structural-bound"
    }
}

fn format_trace_check_json(
    space: &mirspace::ProgramSpace,
    snapshot: &mirsem::TraceSnapshot,
) -> String {
    let trace_by_function = snapshot
        .functions
        .iter()
        .map(|trace| (trace.function, trace))
        .collect::<BTreeMap<_, _>>();
    let observed_call_edges = observed_call_edge_map(snapshot);
    let functions = space
        .function_effect_summaries()
        .into_iter()
        .map(|summary| {
            let function = &space.functions[summary.function.0];
            let trace = trace_by_function.get(&function.id);
            let observed_calls = trace.map(|trace| trace.calls).unwrap_or(0);
            let observed_instructions = trace.map(|trace| trace.executed_instructions).unwrap_or(0);
            let observed_allocations = trace.map(|trace| trace.allocations).unwrap_or(0);
            let observed_reads = trace.map(|trace| trace.memory_reads).unwrap_or(0);
            let observed_writes = trace.map(|trace| trace.memory_writes).unwrap_or(0);
            let observed_returns = trace.map(|trace| trace.returns).unwrap_or(0);
            let observed_traps = trace.map(|trace| trace.traps).unwrap_or(0);
            let call_edges = call_edge_checks_json(space, &summary, &observed_call_edges);
            json!({
                "index": summary.function.0,
                "id": function.id.0,
                "name": function_name(space, summary.function),
                "observed_calls": observed_calls,
                "observed_instructions": observed_instructions,
                "call_edges": call_edges,
                "effects": {
                    "allocates": effect_check_json(summary.allocates, observed_allocations),
                    "reads_memory": effect_check_json(summary.reads_memory, observed_reads),
                    "writes_memory": effect_check_json(summary.writes_memory, observed_writes),
                    "may_trap": effect_check_json(summary.may_trap, observed_traps),
                    "guaranteed_terminates_trivially": {
                        "static": summary.guaranteed_terminates_trivially,
                        "observed_returns": observed_returns
                    }
                }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "kind": "trace-check",
        "module": {
            "name": space.name
        },
        "outcome": trace_outcome_json(&snapshot.outcome),
        "observed_totals": {
            "executed_instructions": snapshot.executed_instruction_count,
            "branches": snapshot.branch_count,
            "calls": snapshot.call_instruction_count,
            "memory_addresses": snapshot.address_instruction_count,
            "allocations": snapshot.allocation_count,
            "memory_reads": snapshot.memory_read_count,
            "memory_writes": snapshot.memory_write_count,
            "returns": snapshot.return_count,
            "traps": snapshot.trap_count,
            "call_edges": snapshot.call_edges.iter().map(|edge| edge.calls).sum::<u64>()
        },
        "functions": functions
    })
    .to_string()
}

fn effect_check_json(static_may: bool, observed_count: u64) -> JsonValue {
    json!({
        "static": static_may,
        "observed": observed_count,
        "status": effect_status(static_may, observed_count > 0)
    })
}

fn trace_outcome_json(outcome: &mirsem::trace::TraceOutcome) -> JsonValue {
    match outcome {
        mirsem::trace::TraceOutcome::NotRun => json!({ "kind": "not-run" }),
        mirsem::trace::TraceOutcome::Returned(_) => json!({ "kind": "returned" }),
        mirsem::trace::TraceOutcome::Trapped(trap) => {
            let (code, name) = trap_info(trap);
            json!({
                "kind": "trapped",
                "code": code,
                "name": name
            })
        }
    }
}

fn format_trace_check(space: &mirspace::ProgramSpace, snapshot: &mirsem::TraceSnapshot) -> String {
    let trace_by_function = snapshot
        .functions
        .iter()
        .map(|trace| (trace.function, trace))
        .collect::<BTreeMap<_, _>>();
    let observed_call_edges = observed_call_edge_map(snapshot);
    let mut out = String::new();
    out.push_str(&format!("trace-check module {}\n", space.name));
    out.push_str(&format!(
        "  outcome: {}\n",
        format_trace_outcome(&snapshot.outcome)
    ));
    out.push_str("  observed totals:\n");
    out.push_str(&format!(
        "    executed_instructions: {}\n",
        snapshot.executed_instruction_count
    ));
    out.push_str(&format!("    branches: {}\n", snapshot.branch_count));
    out.push_str(&format!("    calls: {}\n", snapshot.call_instruction_count));
    out.push_str(&format!(
        "    memory_addresses: {}\n",
        snapshot.address_instruction_count
    ));
    out.push_str(&format!("    allocations: {}\n", snapshot.allocation_count));
    out.push_str(&format!(
        "    memory_reads: {}\n",
        snapshot.memory_read_count
    ));
    out.push_str(&format!(
        "    memory_writes: {}\n",
        snapshot.memory_write_count
    ));
    out.push_str(&format!("    returns: {}\n", snapshot.return_count));
    out.push_str(&format!("    traps: {}\n", snapshot.trap_count));
    out.push_str(&format!(
        "    call_edges: {}\n",
        snapshot
            .call_edges
            .iter()
            .map(|edge| edge.calls)
            .sum::<u64>()
    ));

    for summary in space.function_effect_summaries() {
        let function = &space.functions[summary.function.0];
        let name = function_name(space, summary.function);
        let trace = trace_by_function.get(&function.id);
        let observed_calls = trace.map(|trace| trace.calls).unwrap_or(0);
        let observed_instructions = trace.map(|trace| trace.executed_instructions).unwrap_or(0);
        let observed_allocations = trace.map(|trace| trace.allocations).unwrap_or(0);
        let observed_reads = trace.map(|trace| trace.memory_reads).unwrap_or(0);
        let observed_writes = trace.map(|trace| trace.memory_writes).unwrap_or(0);
        let observed_returns = trace.map(|trace| trace.returns).unwrap_or(0);
        let observed_traps = trace.map(|trace| trace.traps).unwrap_or(0);

        out.push_str(&format!(
            "  fn f{}#{} {}\n",
            summary.function.0, function.id.0, name
        ));
        out.push_str(&format!("    observed_calls: {}\n", observed_calls));
        out.push_str(&format!(
            "    observed_instructions: {}\n",
            observed_instructions
        ));
        let call_edges = call_edge_checks(space, &summary, &observed_call_edges);
        if call_edges.is_empty() {
            out.push_str("    call_edges: -\n");
        } else {
            out.push_str("    call_edges:\n");
            for edge in call_edges {
                out.push_str(&format!(
                    "      {} -> {} static={} observed={} status={}\n",
                    format_function_ref(space, edge.caller),
                    format_function_ref(space, edge.callee),
                    edge.static_edge,
                    edge.observed,
                    effect_status(edge.static_edge, edge.observed > 0)
                ));
            }
        }
        out.push_str(&format!(
            "    allocates: static={} observed={} status={}\n",
            summary.allocates,
            observed_allocations,
            effect_status(summary.allocates, observed_allocations > 0)
        ));
        out.push_str(&format!(
            "    reads_memory: static={} observed={} status={}\n",
            summary.reads_memory,
            observed_reads,
            effect_status(summary.reads_memory, observed_reads > 0)
        ));
        out.push_str(&format!(
            "    writes_memory: static={} observed={} status={}\n",
            summary.writes_memory,
            observed_writes,
            effect_status(summary.writes_memory, observed_writes > 0)
        ));
        out.push_str(&format!(
            "    may_trap: static={} observed={} status={}\n",
            summary.may_trap,
            observed_traps,
            effect_status(summary.may_trap, observed_traps > 0)
        ));
        out.push_str(&format!(
            "    guaranteed_terminates_trivially: static={} observed_returns={}\n",
            summary.guaranteed_terminates_trivially, observed_returns
        ));
    }
    out
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CallEdgeCheck {
    caller: FunctionId,
    callee: FunctionId,
    static_edge: bool,
    observed: u64,
}

fn observed_call_edge_map(
    snapshot: &mirsem::TraceSnapshot,
) -> BTreeMap<(FunctionId, FunctionId), u64> {
    snapshot
        .call_edges
        .iter()
        .map(|edge| ((edge.caller, edge.callee), edge.calls))
        .collect()
}

fn call_edge_checks(
    space: &mirspace::ProgramSpace,
    summary: &mirspace::FunctionEffectSummary,
    observed_call_edges: &BTreeMap<(FunctionId, FunctionId), u64>,
) -> Vec<CallEdgeCheck> {
    let caller_id = space.functions[summary.function.0].id;
    let mut checks = summary
        .calls
        .iter()
        .map(|callee| {
            let callee_id = space.functions[callee.0].id;
            CallEdgeCheck {
                caller: caller_id,
                callee: callee_id,
                static_edge: true,
                observed: observed_call_edges
                    .get(&(caller_id, callee_id))
                    .copied()
                    .unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();
    for (&(observed_caller, observed_callee), &observed) in observed_call_edges {
        if observed_caller == caller_id
            && !checks.iter().any(|check| check.callee == observed_callee)
        {
            checks.push(CallEdgeCheck {
                caller: observed_caller,
                callee: observed_callee,
                static_edge: false,
                observed,
            });
        }
    }
    checks.sort_by_key(|check| (check.caller.0, check.callee.0));
    checks
}

fn call_edge_checks_json(
    space: &mirspace::ProgramSpace,
    summary: &mirspace::FunctionEffectSummary,
    observed_call_edges: &BTreeMap<(FunctionId, FunctionId), u64>,
) -> Vec<JsonValue> {
    call_edge_checks(space, summary, observed_call_edges)
        .into_iter()
        .map(|edge| {
            json!({
                "caller": function_ref_json(space, edge.caller),
                "callee": function_ref_json(space, edge.callee),
                "static": edge.static_edge,
                "observed": edge.observed,
                "status": effect_status(edge.static_edge, edge.observed > 0)
            })
        })
        .collect()
}

fn format_function_ref(space: &mirspace::ProgramSpace, function: FunctionId) -> String {
    let function_ix = space.maps.functions[&function];
    format!(
        "f{}#{} {}",
        function_ix.0,
        function.0,
        function_name(space, function_ix)
    )
}

fn function_ref_json(space: &mirspace::ProgramSpace, function: FunctionId) -> JsonValue {
    let function_ix = space.maps.functions[&function];
    json!({
        "index": function_ix.0,
        "id": function.0,
        "name": function_name(space, function_ix)
    })
}

fn effect_status(static_may: bool, observed: bool) -> &'static str {
    match (static_may, observed) {
        (false, false) => "proven-absent",
        (false, true) => "mismatch",
        (true, false) => "conservative",
        (true, true) => "observed",
    }
}

fn format_trace_outcome(outcome: &mirsem::trace::TraceOutcome) -> String {
    match outcome {
        mirsem::trace::TraceOutcome::NotRun => "not-run".to_string(),
        mirsem::trace::TraceOutcome::Returned(_) => "returned".to_string(),
        mirsem::trace::TraceOutcome::Trapped(trap) => {
            let (code, name) = trap_info(trap);
            format!("trapped {code} {name}")
        }
    }
}

fn format_effect_summaries(space: &mirspace::ProgramSpace) -> String {
    let mut out = String::new();
    out.push_str(&format!("analysis module {}\n", space.name));
    for summary in space.function_effect_summaries() {
        let function = &space.functions[summary.function.0];
        let name = function_name(space, summary.function);
        out.push_str(&format!(
            "  fn f{}#{} {}\n",
            summary.function.0, function.id.0, name
        ));
        out.push_str(&format!("    allocates: {}\n", summary.allocates));
        out.push_str(&format!("    reads_memory: {}\n", summary.reads_memory));
        out.push_str(&format!("    writes_memory: {}\n", summary.writes_memory));
        out.push_str(&format!("    may_trap: {}\n", summary.may_trap));
        out.push_str(&format!("    acyclic_cfg: {}\n", summary.acyclic_cfg));
        out.push_str(&format!(
            "    guaranteed_terminates_trivially: {}\n",
            summary.guaranteed_terminates_trivially
        ));
        out.push_str(&format!("    pure_candidate: {}\n", summary.pure_candidate));
        let calls = summary
            .calls
            .iter()
            .map(|callee| {
                let callee_rec = &space.functions[callee.0];
                format!(
                    "f{}#{} {}",
                    callee.0,
                    callee_rec.id.0,
                    function_name(space, *callee)
                )
            })
            .collect::<Vec<_>>();
        out.push_str(&format!(
            "    calls: {}\n",
            if calls.is_empty() {
                "-".to_string()
            } else {
                calls.join(", ")
            }
        ));
    }
    out
}

fn format_effect_summaries_json(space: &mirspace::ProgramSpace) -> String {
    let functions = space
        .function_effect_summaries()
        .into_iter()
        .map(|summary| {
            let function = &space.functions[summary.function.0];
            let calls = summary
                .calls
                .iter()
                .map(|callee| {
                    let callee_rec = &space.functions[callee.0];
                    json!({
                        "index": callee.0,
                        "id": callee_rec.id.0,
                        "name": function_name(space, *callee)
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "index": summary.function.0,
                "id": function.id.0,
                "name": function_name(space, summary.function),
                "allocates": summary.allocates,
                "reads_memory": summary.reads_memory,
                "writes_memory": summary.writes_memory,
                "may_trap": summary.may_trap,
                "acyclic_cfg": summary.acyclic_cfg,
                "guaranteed_terminates_trivially": summary.guaranteed_terminates_trivially,
                "pure_candidate": summary.pure_candidate,
                "calls": calls
            })
        })
        .collect::<Vec<_>>();
    json!({
        "kind": "analyze",
        "module": {
            "name": space.name
        },
        "functions": functions
    })
    .to_string()
}

fn function_name(space: &mirspace::ProgramSpace, function: mirspace::FunctionIx) -> &str {
    let function_rec = &space.functions[function.0];
    space
        .maps
        .symbols
        .get(&function_rec.symbol)
        .map(|symbol_ix| space.symbols[symbol_ix.0].name.as_str())
        .unwrap_or("<unnamed>")
}

pub fn cmd_lower(
    input_path: &str,
    format_opt: Option<&str>,
    optimize: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let mut lowered = mirplan::lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }
    print!("{}", mirplan::format_lowered(&lowered));
    Ok(())
}

pub fn cmd_bench_load(
    input_path: &str,
    format_opt: Option<&str>,
    iterations: u32,
) -> Result<(), CliError> {
    let format = detect_format(input_path, format_opt)?;
    let start = Instant::now();
    let mut checksum = 0usize;

    match format {
        FileFormat::Text => {
            let text = std::fs::read_to_string(input_path)?;
            for _ in 0..iterations {
                let image = ModuleImage::from_text(&text)?;
                checksum = checksum
                    .wrapping_add(image.module.name.len())
                    .wrapping_add(image.functions.len())
                    .wrapping_add(image.instructions.len());
                black_box(&image);
            }
            print_bench_result("text", iterations, start.elapsed().as_nanos(), checksum);
        }
        FileFormat::Binary => {
            let bytes = std::fs::read(input_path)?;
            for _ in 0..iterations {
                let image = ModuleImage::from_capnp_bytes(&bytes)?;
                checksum = checksum
                    .wrapping_add(image.module.name.len())
                    .wrapping_add(image.functions.len())
                    .wrapping_add(image.instructions.len());
                black_box(&image);
            }
            print_bench_result("binary", iterations, start.elapsed().as_nanos(), checksum);
        }
    }

    Ok(())
}

fn print_bench_result(format: &str, iterations: u32, total_ns: u128, checksum: usize) {
    let avg_ns = total_ns / u128::from(iterations);
    println!("format: {format}");
    println!("iterations: {iterations}");
    println!("total_ns: {total_ns}");
    println!("avg_ns: {avg_ns}");
    println!("checksum: {checksum}");
}

pub fn cmd_compile_c(
    input_path: &str,
    output_path: &str,
    format_opt: Option<&str>,
    entry_name: &str,
    optimize: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let mut lowered = mirplan::lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }

    use mirplan::Backend;
    let backend = mirc0::C11Backend::new(entry_name);
    let c_code = backend.compile(&lowered)?;

    std::fs::write(output_path, c_code)?;
    Ok(())
}

pub fn cmd_compile_rv32i(
    input_path: &str,
    output_path: &str,
    format_opt: Option<&str>,
    optimize: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let mut lowered = mirplan::lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }

    use mirplan::Backend;
    let backend = mirrv32::Riscv32Backend;
    let asm_code = backend
        .compile(&lowered)
        .map_err(|err| CliError::Generic(err.to_string()))?;

    std::fs::write(output_path, asm_code)?;
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DiffOutcome {
    Success(Vec<mirsem::Value>),
    Trap(u32),
}

pub fn cmd_diff(
    input_path: &str,
    format_opt: Option<&str>,
    entry_name: &str,
    keep_temp: bool,
    optimize: bool,
    quiet: bool,
) -> Result<bool, CliError> {
    let image = load_image(input_path, format_opt)?;

    // 1. Run interpreter
    let mut runner = Runner::new(image.clone(), mirsem::ExecutionProfile::default())?;
    let expected = match runner.run_entry_by_name(entry_name, &[]) {
        Ok(res) => DiffOutcome::Success(res.values),
        Err(mirsem::RunError::Trap(trap)) => {
            let (code, _) = trap_info(&trap);
            DiffOutcome::Trap(code)
        }
        Err(err) => {
            return Err(CliError::Generic(format!(
                "Reference interpreter run failed: {:?}",
                err
            )));
        }
    };

    // 2. Generate C
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let mut lowered = mirplan::lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }

    use mirplan::Backend;
    let backend = mirc0::C11Backend::new(entry_name);
    let c_code = backend.compile(&lowered)?;

    // 3. Check for C compiler
    let cc_check = std::process::Command::new("cc").arg("--version").output();
    if cc_check.is_err() {
        if !quiet {
            println!(
                "Host C compiler 'cc' is unavailable. Skipping differential execution verification."
            );
        }
        return Ok(false);
    }

    // 4. Write C source code and compile
    let cur_dir = std::env::current_dir()?;
    let input_name = Path::new(input_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("temp")
        .replace('.', "_");
    let pid = std::process::id();
    let c_path = cur_dir.join(format!("temp_mirtool_diff_{}_{}.c", input_name, pid));
    let bin_path = cur_dir.join(format!("temp_mirtool_diff_{}_{}", input_name, pid));

    std::fs::write(&c_path, c_code)?;

    let mut compile_cmd = std::process::Command::new("cc");
    compile_cmd
        .arg("-O0")
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-o")
        .arg(&bin_path)
        .arg(&c_path);

    let compile_output = compile_cmd.output();
    match compile_output {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !keep_temp {
                    let _ = std::fs::remove_file(&c_path);
                }
                if !quiet {
                    println!("FAIL: C compilation failed:\n{}", stderr);
                }
                return Ok(false);
            }
        }
        Err(err) => {
            if !keep_temp {
                let _ = std::fs::remove_file(&c_path);
            }
            if !quiet {
                println!("FAIL: Failed to run C compiler: {}", err);
            }
            return Ok(false);
        }
    }

    // 5. Run compiled binary
    let run_output = std::process::Command::new(&bin_path).output();
    if !keep_temp {
        let _ = std::fs::remove_file(&c_path);
        let _ = std::fs::remove_file(&bin_path);
    }

    let output = match run_output {
        Ok(o) => o,
        Err(err) => {
            if !quiet {
                println!("FAIL: Failed to execute compiled binary: {}", err);
            }
            return Ok(false);
        }
    };

    // 6. Compare results
    let is_pass = match expected {
        DiffOutcome::Success(expected_values) => {
            if output.status.code() != Some(0) {
                if !quiet {
                    println!(
                        "FAIL: Expected exit code 0 for normal return, got status {:?}",
                        output.status
                    );
                }
                return Ok(false);
            }
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let result_lines = stdout_str
                .lines()
                .filter(|line| line.starts_with("Result: "))
                .map(str::to_string)
                .collect::<Vec<_>>();
            let expected_lines = expected_result_lines(&expected_values);
            if result_lines == expected_lines {
                if !quiet {
                    println!("PASS");
                }
                true
            } else {
                if !quiet {
                    println!(
                        "FAIL: Result mismatch. Expected {:?}, got {:?}",
                        expected_lines, result_lines
                    );
                }
                false
            }
        }
        DiffOutcome::Trap(expected_code) => {
            if output.status.code() != Some(expected_code as i32) {
                if !quiet {
                    println!(
                        "FAIL: Expected exit status to match trap code {}, got status {:?}",
                        expected_code, output.status
                    );
                }
                return Ok(false);
            }
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let trap_line = stderr_str.lines().find(|l| l.starts_with("Trap: "));
            if let Some(line) = trap_line {
                let expected_line = format!("Trap: {} {}", expected_code, trap_name(expected_code));
                if line == expected_line.as_str() {
                    if !quiet {
                        println!("PASS");
                    }
                    true
                } else {
                    if !quiet {
                        println!(
                            "FAIL: Trap line mismatch. Expected '{}', got '{}'",
                            expected_line, line
                        );
                    }
                    false
                }
            } else {
                if !quiet {
                    println!(
                        "FAIL: Expected stderr to contain 'Trap: ' line. Stderr:\n{}",
                        stderr_str
                    );
                }
                false
            }
        }
    };

    Ok(is_pass)
}

fn expected_result_lines(values: &[mirsem::Value]) -> Vec<String> {
    if values.is_empty() {
        return vec!["Result: void".to_string()];
    }
    values.iter().map(expected_value_line).collect()
}

fn expected_value_line(value: &mirsem::Value) -> String {
    match value {
        mirsem::Value::Void => "Result: void".to_string(),
        mirsem::Value::I32(v) => format!("Result: i32 {}", v),
        mirsem::Value::U32(v) => format!("Result: u32 {}", v),
        mirsem::Value::Addr32(v) => format!("Result: addr32 {}", v),
        mirsem::Value::I64(v) => format!("Result: i64 {}", v),
        mirsem::Value::F32(bits) => {
            format!("Result: f32 {} bits=0x{bits:08x}", f32::from_bits(*bits))
        }
        mirsem::Value::F64(bits) => {
            format!("Result: f64 {} bits=0x{bits:016x}", f64::from_bits(*bits))
        }
    }
}

fn first_result_exit_code(values: &[mirsem::Value]) -> i32 {
    match values.first() {
        None | Some(mirsem::Value::Void) => 0,
        Some(mirsem::Value::I32(v)) => *v,
        Some(mirsem::Value::U32(v)) => *v as i32,
        Some(mirsem::Value::Addr32(v)) => *v as i32,
        Some(mirsem::Value::I64(v)) => *v as i32,
        Some(mirsem::Value::F32(bits)) => *bits as i32,
        Some(mirsem::Value::F64(bits)) => *bits as i32,
    }
}

fn print_trace_summary(snapshot: &mirsem::TraceSnapshot) {
    println!("--- Trace Summary ---");
    println!(
        "Executed Instructions: {}",
        snapshot.executed_instruction_count
    );
    println!("Branches: {}", snapshot.branch_count);
    println!("Call Instructions: {}", snapshot.call_instruction_count);
    println!(
        "Address Instructions: {}",
        snapshot.address_instruction_count
    );
    println!(
        "Maximum Call Depth: {}",
        snapshot.maximum_call_depth_reached
    );
    println!("Allocations: {}", snapshot.allocation_count);
    println!("Allocated Bytes: {}", snapshot.allocated_bytes);
    println!("Memory Reads: {}", snapshot.memory_read_count);
    println!("Memory Writes: {}", snapshot.memory_write_count);
    println!("Returns: {}", snapshot.return_count);
    println!("Traps: {}", snapshot.trap_count);
}

pub fn image_to_text(image: &ModuleImage) -> String {
    let mut out = String::new();
    out.push_str("# Note: Decode output is for debugging purposes and is not yet a canonical source format\n");
    out.push_str(&format!("mircap {}\n", image.header.schema_name));
    out.push_str(&format!("version {}\n", image.header.format_version));
    out.push_str(&format!(
        "module {} {}\n",
        image.module.id, image.module.name
    ));

    for ty in &image.types {
        let kind_str = match ty.kind {
            mircap::TypeKind::Void => "void",
            mircap::TypeKind::I32 => "i32",
            mircap::TypeKind::U32 => "u32",
            mircap::TypeKind::Addr32 => "addr32",
            mircap::TypeKind::I64 => "i64",
            mircap::TypeKind::F32 => "f32",
            mircap::TypeKind::F64 => "f64",
            mircap::TypeKind::UnsupportedFloat => "float",
            mircap::TypeKind::UnsupportedLongDouble => "long_double",
            mircap::TypeKind::UnsupportedAggregate => "aggregate",
            mircap::TypeKind::UnsupportedVarargs => "varargs",
            mircap::TypeKind::UnsupportedHostCAbi => "host_c_abi",
        };
        out.push_str(&format!("type {} {}\n", ty.id.0, kind_str));
    }

    for sym in &image.symbols {
        let kind_str = match sym.kind {
            mircap::SymbolKind::Function => "function",
            mircap::SymbolKind::Data => "data",
            mircap::SymbolKind::RuntimeHelper => "runtime_helper",
        };
        out.push_str(&format!("symbol {} {} {}\n", sym.id.0, sym.name, kind_str));
    }

    fn fmt_type_list(list: &[mircap::ids::TypeId]) -> String {
        if list.is_empty() {
            "-".to_string()
        } else {
            list.iter()
                .map(|t| t.0.to_string())
                .collect::<Vec<_>>()
                .join(",")
        }
    }

    for func in &image.functions {
        out.push_str(&format!(
            "function {} {} {} {} {} {} {}\n",
            func.id.0,
            func.symbol.0,
            fmt_type_list(&func.params),
            fmt_type_list(&func.results),
            func.value_count,
            func.flags,
            fmt_type_list(&func.value_types)
        ));
        for block_id in &func.blocks {
            out.push_str(&format!("func_block {} {}\n", func.id.0, block_id.0));
        }
    }

    for ds in &image.data_segments {
        fn fmt_bytes(bytes: &[u8]) -> String {
            if bytes.is_empty() {
                "-".to_string()
            } else {
                bytes
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>()
            }
        }
        out.push_str(&format!(
            "data {} {} {} {}\n",
            ds.symbol.0,
            ds.offset,
            fmt_bytes(&ds.bytes),
            ds.zero_fill
        ));
    }

    for block in &image.blocks {
        let insn_ids = block
            .instructions
            .iter()
            .map(|id| id.0.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "block {} {} {}\n",
            block.id.0, block.parent.0, insn_ids
        ));
    }

    for insn in &image.instructions {
        let opcode_name = match insn.opcode {
            mircap::Opcode::ConstI32 => "const_i32",
            mircap::Opcode::ConstU32 => "const_u32",
            mircap::Opcode::Copy => "copy",
            mircap::Opcode::AddI32 => "add_i32",
            mircap::Opcode::SubI32 => "sub_i32",
            mircap::Opcode::MulI32 => "mul_i32",
            mircap::Opcode::EqI32 => "eq_i32",
            mircap::Opcode::NeI32 => "ne_i32",
            mircap::Opcode::LtI32 => "lt_i32",
            mircap::Opcode::AddU32 => "add_u32",
            mircap::Opcode::SubU32 => "sub_u32",
            mircap::Opcode::MulU32 => "mul_u32",
            mircap::Opcode::EqU32 => "eq_u32",
            mircap::Opcode::NeU32 => "ne_u32",
            mircap::Opcode::LtU32 => "lt_u32",
            mircap::Opcode::LeU32 => "le_u32",
            mircap::Opcode::GtU32 => "gt_u32",
            mircap::Opcode::GeU32 => "ge_u32",
            mircap::Opcode::Branch => "branch",
            mircap::Opcode::BranchIf => "branch_if",
            mircap::Opcode::Call => "call",
            mircap::Opcode::Ret => "ret",
            mircap::Opcode::Trap => "trap",
            mircap::Opcode::Alloc => "alloc",
            mircap::Opcode::LoadI32 => "load_i32",
            mircap::Opcode::LoadU32 => "load_u32",
            mircap::Opcode::StoreI32 => "store_i32",
            mircap::Opcode::StoreU32 => "store_u32",
            mircap::Opcode::LoadU8 => "load_u8",
            mircap::Opcode::StoreU8 => "store_u8",
            mircap::Opcode::AddrAdd => "addr_add",
            mircap::Opcode::DataAddr => "data_addr",
            mircap::Opcode::ConstI64 => "const_i64",
            mircap::Opcode::AddI64 => "add_i64",
            mircap::Opcode::SubI64 => "sub_i64",
            mircap::Opcode::MulI64 => "mul_i64",
            mircap::Opcode::EqI64 => "eq_i64",
            mircap::Opcode::NeI64 => "ne_i64",
            mircap::Opcode::LtI64 => "lt_i64",
            mircap::Opcode::LoadI64 => "load_i64",
            mircap::Opcode::StoreI64 => "store_i64",
            mircap::Opcode::ConstF32 => "const_f32",
            mircap::Opcode::ConstF64 => "const_f64",
            mircap::Opcode::AddF32 => "add_f32",
            mircap::Opcode::SubF32 => "sub_f32",
            mircap::Opcode::MulF32 => "mul_f32",
            mircap::Opcode::DivF32 => "div_f32",
            mircap::Opcode::NegF32 => "neg_f32",
            mircap::Opcode::EqF32 => "eq_f32",
            mircap::Opcode::NeF32 => "ne_f32",
            mircap::Opcode::LtF32 => "lt_f32",
            mircap::Opcode::LeF32 => "le_f32",
            mircap::Opcode::GtF32 => "gt_f32",
            mircap::Opcode::GeF32 => "ge_f32",
            mircap::Opcode::AddF64 => "add_f64",
            mircap::Opcode::SubF64 => "sub_f64",
            mircap::Opcode::MulF64 => "mul_f64",
            mircap::Opcode::DivF64 => "div_f64",
            mircap::Opcode::NegF64 => "neg_f64",
            mircap::Opcode::EqF64 => "eq_f64",
            mircap::Opcode::NeF64 => "ne_f64",
            mircap::Opcode::LtF64 => "lt_f64",
            mircap::Opcode::LeF64 => "le_f64",
            mircap::Opcode::GtF64 => "gt_f64",
            mircap::Opcode::GeF64 => "ge_f64",
            mircap::Opcode::I32ToF32 => "i32_to_f32",
            mircap::Opcode::F32ToI32 => "f32_to_i32",
            mircap::Opcode::I32ToF64 => "i32_to_f64",
            mircap::Opcode::F64ToI32 => "f64_to_i32",
            mircap::Opcode::F32ToF64 => "f32_to_f64",
            mircap::Opcode::F64ToF32 => "f64_to_f32",
            mircap::Opcode::UnsupportedIndirectCall => "indirect_call",
        };

        let mut parts = Vec::new();
        for &res in &insn.results {
            parts.push(format!("r:{}", res.0));
        }
        for op in &insn.operands {
            let op_str = match op {
                mircap::Operand::Value(val) => format!("v:{}", val.0),
                mircap::Operand::ImmI32(val) => format!("i:{}", val),
                mircap::Operand::ImmU32(val) => format!("u:{}", val),
                mircap::Operand::Block(val) => format!("b:{}", val.0),
                mircap::Operand::Function(val) => format!("f:{}", val.0),
                mircap::Operand::Symbol(val) => format!("s:{}", val.0),
                mircap::Operand::Type(val) => format!("t:{}", val.0),
                mircap::Operand::ImmI64(val) => format!("l:{}", val),
                mircap::Operand::ImmF32(bits) => format!("f32:{}", f32::from_bits(*bits)),
                mircap::Operand::ImmF64(bits) => format!("f64:{}", f64::from_bits(*bits)),
            };
            parts.push(op_str);
        }
        if parts.is_empty() {
            out.push_str(&format!("insn {} {}\n", insn.id.0, opcode_name));
        } else {
            out.push_str(&format!(
                "insn {} {} {}\n",
                insn.id.0,
                opcode_name,
                parts.join(" ")
            ));
        }
    }

    out
}

pub fn cmd_diff_upstream(
    input_path: &str,
    format_opt: Option<&str>,
    entry_name: &str,
    keep_temp: bool,
    optimize: bool,
    quiet: bool,
) -> Result<bool, CliError> {
    let image = load_image(input_path, format_opt)?;

    // 1. Run interpreter
    let mut runner = Runner::new(image.clone(), mirsem::ExecutionProfile::default())?;
    let expected = match runner.run_entry_by_name(entry_name, &[]) {
        Ok(res) => DiffOutcome::Success(res.values),
        Err(mirsem::RunError::Trap(trap)) => {
            let (code, _) = trap_info(&trap);
            DiffOutcome::Trap(code)
        }
        Err(err) => {
            return Err(CliError::Generic(format!(
                "Reference interpreter run failed: {:?}",
                err
            )));
        }
    };

    // 2. Generate original MIR
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let mut lowered = mirplan::lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }

    let mir_code = translate_to_upstream_mir(&lowered, entry_name);

    // 3. Write MIR source code and compile
    let cur_dir = std::env::current_dir()?;
    let input_name = Path::new(input_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("temp")
        .replace('.', "_");
    let mir_path = cur_dir.join(format!("temp_mirtool_upstream_{}.mir", input_name));
    let bmir_path = cur_dir.join(format!("temp_mirtool_upstream_{}.bmir", input_name));

    std::fs::write(&mir_path, mir_code)?;

    let m2b_path = "/home/john/project/mir-preservation/git/mir-restored/m2b";
    let mir_bin_run_path = "/home/john/project/mir-preservation/git/mir-restored/mir-bin-run";

    let mut compile_cmd = std::process::Command::new(m2b_path);
    compile_cmd.stdin(std::fs::File::open(&mir_path)?);
    compile_cmd.stdout(std::fs::File::create(&bmir_path)?);

    let compile_output = compile_cmd.output();
    match compile_output {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !keep_temp {
                    let _ = std::fs::remove_file(&mir_path);
                    let _ = std::fs::remove_file(&bmir_path);
                }
                if !quiet {
                    println!("FAIL: m2b compilation failed:\n{}", stderr);
                }
                return Ok(false);
            }
        }
        Err(err) => {
            if !keep_temp {
                let _ = std::fs::remove_file(&mir_path);
                let _ = std::fs::remove_file(&bmir_path);
            }
            if !quiet {
                println!("FAIL: Failed to run m2b compiler: {}", err);
            }
            return Ok(false);
        }
    }

    // 4. Run compiled binary with mir-bin-run
    let run_output = std::process::Command::new(mir_bin_run_path)
        .arg(&bmir_path)
        .arg("dummy_name")
        .output();

    if !keep_temp {
        let _ = std::fs::remove_file(&mir_path);
        let _ = std::fs::remove_file(&bmir_path);
    }

    let output = match run_output {
        Ok(o) => o,
        Err(err) => {
            if !quiet {
                println!(
                    "FAIL: Failed to execute compiled binary under mir-bin-run: {}",
                    err
                );
            }
            return Ok(false);
        }
    };

    let exit_code = output.status.code();

    // 5. Compare exit codes
    let is_pass = match expected {
        DiffOutcome::Success(expected_values) => {
            let expected_code = first_result_exit_code(&expected_values);
            let expected_exit_status = (expected_code & 0xff) as i32;
            let actual_exit_status = exit_code.map(|c| c & 0xff);
            if actual_exit_status == Some(expected_exit_status) {
                if !quiet {
                    println!("PASS");
                }
                true
            } else {
                if !quiet {
                    println!(
                        "FAIL: Result mismatch. Expected exit code {} (masked: {}), got {:?}",
                        expected_code, expected_exit_status, exit_code
                    );
                }
                false
            }
        }
        DiffOutcome::Trap(expected_trap_code) => {
            let expected_exit_status = (expected_trap_code & 0xff) as i32;
            let actual_exit_status = exit_code.map(|c| c & 0xff);
            if actual_exit_status == Some(expected_exit_status) {
                if !quiet {
                    println!("PASS");
                }
                true
            } else {
                if !quiet {
                    println!(
                        "FAIL: Trap mismatch. Expected exit status to match trap code {} (masked: {}), got {:?}",
                        expected_trap_code, expected_exit_status, exit_code
                    );
                }
                false
            }
        }
    };

    Ok(is_pass)
}

fn map_type(kind: mircap::TypeKind) -> &'static str {
    match kind {
        mircap::TypeKind::Void => "void",
        mircap::TypeKind::I32 => "i32",
        mircap::TypeKind::U32 => "u32",
        mircap::TypeKind::Addr32 => "p",
        mircap::TypeKind::I64 => "i64",
        _ => "i64",
    }
}

pub fn translate_to_upstream_mir(program: &LoweredProgram, entry_name: &str) -> String {
    let mut out = String::new();

    // Module header
    out.push_str(&format!("{}: module\n", program.module_name));

    // Standard imports and prototypes
    out.push_str("import malloc, abort, exit\n");
    out.push_str("proto_malloc: proto p, i64:size\n");
    out.push_str("proto_abort: proto\n");
    out.push_str("proto_exit: proto i64:code\n\n");

    // Globals
    out.push_str("g_heap_ptr: i64 0\n");
    out.push_str("g_memory: bss 1048576\n\n");

    // Emit data segment declarations
    for (idx, segment) in program.data_segments.iter().enumerate() {
        out.push_str(&format!("data_seg_{}: u8 ", idx));
        if segment.bytes.is_empty() {
            out.push_str("0");
        } else {
            let byte_strs = segment
                .bytes
                .iter()
                .map(|b| format!("0x{b:02x}"))
                .collect::<Vec<_>>();
            out.push_str(&byte_strs.join(", "));
        }
        out.push_str("\n");
    }
    out.push('\n');

    // Emit static safety helpers
    out.push_str(HELPER_FUNCTIONS);

    // Emit dynamic init_data_segments function
    out.push_str("init_data_segments: func\n");
    out.push_str(
        "                    local i64:mem_addr, i64:seg_addr, i64:i, i64:temp, i64:val\n",
    );
    for (idx, segment) in program.data_segments.iter().enumerate() {
        let len = segment.bytes.len();
        out.push_str(&format!("                    # Segment {}\n", idx));
        if len > 0 {
            out.push_str("                    mov i, 0\n");
            out.push_str(&format!("L_loop_seg_{}:\n", idx));
            out.push_str(&format!(
                "                    bge L_end_seg_{}, i, {}\n",
                idx, len
            ));
            out.push_str(&format!(
                "                    add temp, {}, i\n",
                segment.offset
            ));
            out.push_str(&format!(
                "                    mov seg_addr, data_seg_{}\n",
                idx
            ));
            out.push_str("                    add seg_addr, seg_addr, i\n");
            out.push_str("                    mov val, u8:0(seg_addr)\n");
            out.push_str("                    mov mem_addr, g_memory\n");
            out.push_str("                    add mem_addr, mem_addr, temp\n");
            out.push_str("                    mov u8:0(mem_addr), val\n");
            out.push_str("                    add i, i, 1\n");
            out.push_str(&format!("                    jmp L_loop_seg_{}\n", idx));
            out.push_str(&format!("L_end_seg_{}:\n", idx));
        }
        if segment.zero_fill > 0 {
            let zero_start = segment.offset as usize + len;
            out.push_str("                    mov i, 0\n");
            out.push_str(&format!("L_zero_loop_seg_{}:\n", idx));
            out.push_str(&format!(
                "                    bge L_zero_end_seg_{}, i, {}\n",
                idx, segment.zero_fill
            ));
            out.push_str(&format!(
                "                    add temp, {}, i\n",
                zero_start
            ));
            out.push_str("                    mov mem_addr, g_memory\n");
            out.push_str("                    add mem_addr, mem_addr, temp\n");
            out.push_str("                    mov u8:0(mem_addr), 0\n");
            out.push_str("                    add i, i, 1\n");
            out.push_str(&format!(
                "                    jmp L_zero_loop_seg_{}\n",
                idx
            ));
            out.push_str(&format!("L_zero_end_seg_{}:\n", idx));
        }
    }
    out.push_str("                    ret\n");
    out.push_str("                    endfunc\n\n");

    // Prototypes for other functions in the module
    for func in &program.functions {
        let name = if func.name == entry_name {
            "main"
        } else {
            &func.name
        };
        let results_str = func
            .results
            .iter()
            .map(|t| map_type(*t))
            .collect::<Vec<_>>()
            .join(", ");
        let params_str = func
            .params
            .iter()
            .map(|p| format!("{}:v_{}", map_type(p.type_kind), p.id.0))
            .collect::<Vec<_>>()
            .join(", ");

        let signature = if results_str.is_empty() {
            if params_str.is_empty() {
                "".to_string()
            } else {
                format!(" {}", params_str)
            }
        } else {
            if params_str.is_empty() {
                format!(" {}", results_str)
            } else {
                format!(" {}, {}", results_str, params_str)
            }
        };
        out.push_str(&format!("proto_{}: proto{}\n", name, signature));
    }
    out.push('\n');

    // Translate functions
    let mut sorted_functions: Vec<_> = program.functions.iter().collect();
    sorted_functions.sort_by_key(|f| f.name == entry_name);
    for func in sorted_functions {
        let name = if func.name == entry_name {
            "main"
        } else {
            &func.name
        };
        let results_str = func
            .results
            .iter()
            .map(|t| map_type(*t))
            .collect::<Vec<_>>()
            .join(", ");
        let params_str = func
            .params
            .iter()
            .map(|p| format!("{}:v_{}", map_type(p.type_kind), p.id.0))
            .collect::<Vec<_>>()
            .join(", ");

        let signature = if results_str.is_empty() {
            if params_str.is_empty() {
                "".to_string()
            } else {
                format!(" {}", params_str)
            }
        } else {
            if params_str.is_empty() {
                format!(" {}", results_str)
            } else {
                format!(" {}, {}", results_str, params_str)
            }
        };
        out.push_str(&format!("{}: func{}\n", name, signature));

        // Declare local variables (all used registers except parameters, type i64)
        let param_ids: std::collections::HashSet<u32> =
            func.params.iter().map(|p| p.id.0).collect();
        let mut local_ids = std::collections::HashSet::new();
        for block in &func.blocks {
            for insn in &block.instructions {
                for r in &insn.writes {
                    if !param_ids.contains(&r.id.0) {
                        local_ids.insert(r.id.0);
                    }
                }
                for r in &insn.reads {
                    if !param_ids.contains(&r.id.0) {
                        local_ids.insert(r.id.0);
                    }
                }
                for op in &insn.operands {
                    if let LoweredOperand::Value(val) = op {
                        if !param_ids.contains(&val.id.0) {
                            local_ids.insert(val.id.0);
                        }
                    }
                }
            }
        }

        let mut sorted_ids: Vec<_> = local_ids.into_iter().collect();
        sorted_ids.sort();
        let mut decls = sorted_ids
            .iter()
            .map(|id| format!("i64:v_{}", id))
            .collect::<Vec<_>>();
        decls.push("i64:addr_add_temp".to_string());
        let local_declarations = decls.join(", ");
        out.push_str(&format!(
            "                    local {}\n",
            local_declarations
        ));

        // If this is the entry function ("main"), call init_data_segments first
        if name == "main" {
            out.push_str("                    call proto_init_data_segments, init_data_segments\n");
        }

        // If the first block is not the entry block, jump to the entry block
        if let Some(first_block) = func.blocks.first() {
            if first_block.label.id != func.entry.id {
                out.push_str(&format!("                    jmp L_{}\n", func.entry.id.0));
            }
        }

        // Translate blocks
        for block in &func.blocks {
            out.push_str(&format!("L_{}:\n", block.label.id.0));
            for insn in &block.instructions {
                out.push_str("                    ");
                let insn_str = translate_instruction(insn, entry_name, &program.data_segments);
                out.push_str(&insn_str);
                out.push('\n');
            }
        }
        out.push_str("                    endfunc\n\n");
    }

    out.push_str("endmodule\n");
    out
}

fn translate_instruction(
    insn: &LoweredInstruction,
    entry_name: &str,
    data_segments: &[DataSegmentPlan],
) -> String {
    let dest_str = if !insn.writes.is_empty() {
        format!("v_{}", insn.writes[0].id.0)
    } else {
        String::new()
    };

    let format_op = |op: &LoweredOperand| -> String {
        match op {
            LoweredOperand::Value(val) => format!("v_{}", val.id.0),
            LoweredOperand::ImmI32(imm) => imm.to_string(),
            LoweredOperand::ImmU32(imm) => imm.to_string(),
            LoweredOperand::ImmI64(imm) => imm.to_string(),
            LoweredOperand::Block(lbl) => format!("L_{}", lbl.id.0),
            LoweredOperand::Function(f) => {
                let name = if f.name == entry_name {
                    "main"
                } else {
                    &f.name
                };
                name.to_string()
            }
            LoweredOperand::Symbol { name, .. } => name.clone(),
            _ => String::new(),
        }
    };

    match insn.opcode {
        Opcode::ConstI32 | Opcode::ConstU32 | Opcode::ConstI64 => {
            let val = format_op(&insn.operands[0]);
            format!("mov {}, {}", dest_str, val)
        }
        Opcode::Copy => {
            let val = format_op(&insn.operands[0]);
            format!("mov {}, {}", dest_str, val)
        }
        Opcode::AddI32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("adds {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::SubI32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("subs {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::MulI32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("muls {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::AddU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("adds {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::SubU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("subs {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::MulU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("umuls {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::EqI32 | Opcode::EqU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("eqs {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::NeI32 | Opcode::NeU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("nes {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::LtI32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("lts {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::LtU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("ults {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::LeU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("ules {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::GtU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("ugts {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::GeU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("uges {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::Branch => {
            let lbl = format_op(&insn.operands[0]);
            format!("jmp {}", lbl)
        }
        Opcode::BranchIf => {
            let cond = format_op(&insn.operands[0]);
            let true_lbl = format_op(&insn.operands[1]);
            let false_lbl = format_op(&insn.operands[2]);
            format!(
                "bt {}, {}\n                    jmp {}",
                true_lbl, cond, false_lbl
            )
        }
        Opcode::Ret => {
            let read_strs: Vec<String> =
                insn.reads.iter().map(|v| format!("v_{}", v.id.0)).collect();
            if read_strs.is_empty() {
                "ret".to_string()
            } else {
                format!("ret {}", read_strs.join(", "))
            }
        }
        Opcode::Call => {
            let callee_ref = match insn
                .operands
                .iter()
                .find(|op| matches!(op, LoweredOperand::Function(_)))
            {
                Some(LoweredOperand::Function(f)) => f,
                _ => panic!("Call instruction must have function callee"),
            };
            let callee_name = if callee_ref.name == entry_name {
                "main"
            } else {
                &callee_ref.name
            };

            let args: Vec<String> = insn
                .operands
                .iter()
                .filter(|op| !matches!(op, LoweredOperand::Function(_)))
                .map(|op| format_op(op))
                .collect();

            let mut parts = vec![format!("proto_{}", callee_name), callee_name.to_string()];
            if !dest_str.is_empty() {
                parts.push(dest_str);
            }
            parts.extend(args);
            format!("call {}", parts.join(", "))
        }
        Opcode::Trap => "call proto_exit, exit, 3".to_string(),
        Opcode::Alloc => {
            let size = format_op(&insn.operands[0]);
            let align = format_op(&insn.operands[1]);
            format!(
                "call proto_mir_alloc, mir_alloc, {}, {}, {}",
                dest_str, size, align
            )
        }
        Opcode::LoadI32 => {
            let addr = format_op(&insn.operands[0]);
            format!(
                "call proto_mir_load_i32, mir_load_i32, {}, {}",
                dest_str, addr
            )
        }
        Opcode::LoadU32 => {
            let addr = format_op(&insn.operands[0]);
            format!(
                "call proto_mir_load_u32, mir_load_u32, {}, {}",
                dest_str, addr
            )
        }
        Opcode::StoreI32 => {
            let addr = format_op(&insn.operands[0]);
            let val = format_op(&insn.operands[1]);
            format!("call proto_mir_store_i32, mir_store_i32, {}, {}", addr, val)
        }
        Opcode::StoreU32 => {
            let addr = format_op(&insn.operands[0]);
            let val = format_op(&insn.operands[1]);
            format!("call proto_mir_store_u32, mir_store_u32, {}, {}", addr, val)
        }
        Opcode::LoadU8 => {
            let addr = format_op(&insn.operands[0]);
            format!(
                "call proto_mir_load_u8, mir_load_u8, {}, {}",
                dest_str, addr
            )
        }
        Opcode::StoreU8 => {
            let addr = format_op(&insn.operands[0]);
            let val = format_op(&insn.operands[1]);
            format!("call proto_mir_store_u8, mir_store_u8, {}, {}", addr, val)
        }
        Opcode::AddrAdd => {
            let base = format_op(&insn.operands[0]);
            let offset = format_op(&insn.operands[1]);
            let insn_id = insn.id.0;
            format!(
                "mov addr_add_temp, {base}\n                    adds {dest}, addr_add_temp, {offset}\n                    ublts L_overflow_addradd_insn_{insn_id}, {dest}, addr_add_temp\n                    uext32 {dest}, {dest}\n                    jmp L_ok_addradd_insn_{insn_id}\nL_overflow_addradd_insn_{insn_id}:\n                    call proto_exit, exit, 17\nL_ok_addradd_insn_{insn_id}:",
                dest = dest_str,
                base = base,
                offset = offset,
                insn_id = insn_id
            )
        }
        Opcode::DataAddr => {
            let sym_name = match &insn.operands[0] {
                LoweredOperand::Symbol { name, .. } => name.clone(),
                _ => String::new(),
            };
            let offset = format_op(&insn.operands[1]);
            let ds = data_segments
                .iter()
                .find(|ds| ds.name == sym_name)
                .expect("Data segment must exist for DataAddr instruction");
            let ds_len = ds.bytes.len() as u32 + ds.zero_fill;
            format!(
                "call proto_mir_data_addr, mir_data_addr, {}, {}, {}, {}",
                dest_str, ds.offset, offset, ds_len
            )
        }
        Opcode::AddI64 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("add {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::SubI64 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("sub {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::MulI64 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("mul {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::EqI64 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("eq {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::NeI64 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("ne {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::LtI64 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("lt {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::LoadI64 => {
            let addr = format_op(&insn.operands[0]);
            format!(
                "call proto_mir_load_i64, mir_load_i64, {}, {}",
                dest_str, addr
            )
        }
        Opcode::StoreI64 => {
            let addr = format_op(&insn.operands[0]);
            let val = format_op(&insn.operands[1]);
            format!("call proto_mir_store_i64, mir_store_i64, {}, {}", addr, val)
        }
        _ => String::new(),
    }
}

const HELPER_FUNCTIONS: &str = r#"proto_init_data_segments: proto
proto_mir_alloc: proto i64, i64:size, i64:align
proto_mir_load_i32: proto i64, i64:addr
proto_mir_load_u32: proto i64, i64:addr
proto_mir_store_i32: proto i64:addr, i64:val
proto_mir_store_u32: proto i64:addr, i64:val
proto_mir_load_u8: proto i64, i64:addr
proto_mir_store_u8: proto i64:addr, i64:val
proto_mir_load_i64: proto i64, i64:addr
proto_mir_store_i64: proto i64:addr, i64:val
proto_mir_data_addr: proto i64, i64:base, i64:offset, i64:len

mir_trap: func i64:code
          call proto_exit, exit, code
          ret
          endfunc

mir_alloc: func i64, i64:size, i64:align
           local i64:mask, i64:heap_ptr_addr, i64:heap_val, i64:aligned, i64:end, i64:temp, i64:limit, i64:not_mask
           bne L_align_ok_1_alloc, align, 0
           call proto_exit, exit, 16
L_align_ok_1_alloc:
           sub temp, align, 1
           and temp, align, temp
           beq L_align_ok_2_alloc, temp, 0
           call proto_exit, exit, 16
L_align_ok_2_alloc:
           sub mask, align, 1
           mov heap_ptr_addr, g_heap_ptr
           mov heap_val, i64:0(heap_ptr_addr)
           mov limit, 4294967295
           sub limit, limit, mask
           ubgt L_oom_1_alloc, heap_val, limit
           jmp L_heap_ok_1_alloc
L_oom_1_alloc:
           call proto_exit, exit, 11
L_heap_ok_1_alloc:
           add aligned, heap_val, mask
           xor not_mask, mask, -1
           and aligned, aligned, not_mask
           mov limit, 4294967295
           sub limit, limit, aligned
           ubgt L_oom_2_alloc, size, limit
           jmp L_heap_ok_2_alloc
L_oom_2_alloc:
           call proto_exit, exit, 11
L_heap_ok_2_alloc:
           add end, aligned, size
           ubgt L_collision_alloc, end, 983040
           jmp L_heap_ok_3_alloc
L_collision_alloc:
           call proto_exit, exit, 12
L_heap_ok_3_alloc:
           mov i64:0(heap_ptr_addr), end
           uext32 aligned, aligned
           ret aligned
           endfunc

mir_load_i32: func i64, i64:addr
              local i64:rem, i64:mem_addr, i64:val
              mod rem, addr, 4
              beq L_align_ok_load_i32, rem, 0
              call proto_exit, exit, 15
L_align_ok_load_i32:
              ubgt L_bounds_ok_load_i32, addr, 1048572
              jmp L_bounds_ok2_load_i32
L_bounds_ok_load_i32:
              call proto_exit, exit, 13
L_bounds_ok2_load_i32:
              mov mem_addr, g_memory
              add mem_addr, mem_addr, addr
              mov val, i32:0(mem_addr)
              ret val
              endfunc

mir_load_u32: func i64, i64:addr
              local i64:rem, i64:mem_addr, i64:val
              mod rem, addr, 4
              beq L_align_ok_load_u32, rem, 0
              call proto_exit, exit, 15
L_align_ok_load_u32:
              ubgt L_bounds_ok_load_u32, addr, 1048572
              jmp L_bounds_ok2_load_u32
L_bounds_ok_load_u32:
              call proto_exit, exit, 13
L_bounds_ok2_load_u32:
              mov mem_addr, g_memory
              add mem_addr, mem_addr, addr
              mov val, u32:0(mem_addr)
              ret val
              endfunc

mir_store_i32: func i64:addr, i64:val
               local i64:rem, i64:mem_addr
               mod rem, addr, 4
               beq L_align_ok_store_i32, rem, 0
               call proto_exit, exit, 16
L_align_ok_store_i32:
               ubgt L_bounds_ok_store_i32, addr, 1048572
               jmp L_bounds_ok2_store_i32
L_bounds_ok_store_i32:
               call proto_exit, exit, 14
L_bounds_ok2_store_i32:
               mov mem_addr, g_memory
               add mem_addr, mem_addr, addr
               mov i32:0(mem_addr), val
               ret
               endfunc

mir_store_u32: func i64:addr, i64:val
               local i64:rem, i64:mem_addr
               mod rem, addr, 4
               beq L_align_ok_store_u32, rem, 0
               call proto_exit, exit, 16
L_align_ok_store_u32:
               ubgt L_bounds_ok_store_u32, addr, 1048572
               jmp L_bounds_ok2_store_u32
L_bounds_ok_store_u32:
               call proto_exit, exit, 14
L_bounds_ok2_store_u32:
               mov mem_addr, g_memory
               add mem_addr, mem_addr, addr
               mov u32:0(mem_addr), val
               ret
               endfunc

mir_load_u8: func i64, i64:addr
             local i64:mem_addr, i64:val
             ubgt L_bounds_ok_load_u8, addr, 1048575
             jmp L_bounds_ok2_load_u8
L_bounds_ok_load_u8:
             call proto_exit, exit, 13
L_bounds_ok2_load_u8:
             mov mem_addr, g_memory
             add mem_addr, mem_addr, addr
             mov val, u8:0(mem_addr)
             ret val
             endfunc

mir_store_u8: func i64:addr, i64:val
              local i64:mem_addr
              ubgt L_bounds_ok_store_u8, addr, 1048575
              jmp L_bounds_ok2_store_u8
L_bounds_ok_store_u8:
              call proto_exit, exit, 14
L_bounds_ok2_store_u8:
              mov mem_addr, g_memory
              add mem_addr, mem_addr, addr
              mov u8:0(mem_addr), val
              ret
              endfunc

mir_load_i64: func i64, i64:addr
              local i64:rem, i64:mem_addr, i64:val
              mod rem, addr, 8
              beq L_align_ok_load_i64, rem, 0
              call proto_exit, exit, 15
L_align_ok_load_i64:
              ubgt L_bounds_ok_load_i64, addr, 1048568
              jmp L_bounds_ok2_load_i64
L_bounds_ok_load_i64:
              call proto_exit, exit, 13
L_bounds_ok2_load_i64:
              mov mem_addr, g_memory
              add mem_addr, mem_addr, addr
              mov val, i64:0(mem_addr)
              ret val
              endfunc

mir_store_i64: func i64:addr, i64:val
               local i64:rem, i64:mem_addr
               mod rem, addr, 8
               beq L_align_ok_store_i64, rem, 0
               call proto_exit, exit, 16
L_align_ok_store_i64:
               ubgt L_bounds_ok_store_i64, addr, 1048568
               jmp L_bounds_ok2_store_i64
L_bounds_ok_store_i64:
               call proto_exit, exit, 14
L_bounds_ok2_store_i64:
               mov mem_addr, g_memory
               add mem_addr, mem_addr, addr
               mov i64:0(mem_addr), val
               ret
               endfunc

mir_data_addr: func i64, i64:base, i64:offset, i64:len
               local i64:limit, i64:res
               ubgt L_bounds_ok_data_addr, offset, len
               jmp L_bounds_ok2_data_addr
L_bounds_ok_data_addr:
               call proto_exit, exit, 13
L_bounds_ok2_data_addr:
               mov limit, 4294967295
               sub limit, limit, offset
               ubgt L_overflow_data_addr, base, limit
               jmp L_overflow_ok_data_addr
L_overflow_data_addr:
               call proto_exit, exit, 17
L_overflow_ok_data_addr:
               add res, base, offset
               uext32 res, res
               ret res
               endfunc

"#;

pub fn cmd_diff_rv32i(
    input_path: &str,
    format_opt: Option<&str>,
    keep_temp: bool,
    optimize: bool,
    quiet: bool,
) -> Result<bool, CliError> {
    use std::os::unix::process::ExitStatusExt;

    let image = load_image(input_path, format_opt)?;

    // 1. Run interpreter
    let mut runner = Runner::new(image.clone(), mirsem::ExecutionProfile::default())?;
    let expected = match runner.run_entry_by_name("main", &[]) {
        Ok(res) => DiffOutcome::Success(res.values),
        Err(mirsem::RunError::Trap(trap)) => {
            let (code, _) = trap_info(&trap);
            DiffOutcome::Trap(code)
        }
        Err(err) => {
            return Err(CliError::Generic(format!(
                "Reference interpreter run failed: {:?}",
                err
            )));
        }
    };

    // 2. Generate RV32I assembly
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let mut lowered = mirplan::lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }

    use mirplan::Backend;
    let backend = mirrv32::Riscv32Backend;
    let generated_asm = backend
        .compile(&lowered)
        .map_err(|err| CliError::Generic(err.to_string()))?;

    // Append runtime stub and custom mir_alloc
    let mut full_asm = String::new();
    full_asm.push_str(&generated_asm);
    full_asm.push_str(
        r#"
.section .text
.global _start
_start:
    jal ra, mir_fn_1
    # Exit syscall (sys_exit is 93 on RISC-V)
    li a7, 93
    ecall

.global mir_alloc
mir_alloc:
    # a0 = size, a1 = align
    la t0, heap_ptr
    lw t1, 0(t0)          # t1 = current heap_ptr
    
    # Align: mask = a1 - 1
    addi t2, a1, -1       # t2 = mask
    add t1, t1, t2        # t1 = heap_ptr + mask
    not t2, t2            # t2 = ~mask
    and t1, t1, t2        # t1 = aligned heap_ptr
    
    la t3, heap_buffer
    li t4, 1048576        # 1MB size limit
    add t3, t3, t4        # t3 = heap_buffer + 1MB
    
    add t4, t1, a0        # t4 = new heap_ptr
    bgtu t4, t3, .Loom
    
    # Update heap_ptr
    sw t4, 0(t0)
    # Return aligned address in a0
    mv a0, t1
    ret
    
.Loom:
    # Exit with OutOfMemory code 11
    li a0, 11
    li a7, 93
    ecall

.section .data
.align 4
heap_ptr:
    .word heap_buffer

.section .bss
.align 16
heap_buffer:
    .zero 1048576          # 1MB heap buffer
"#,
    );

    // 3. Check for tools
    let gcc_check = std::process::Command::new("riscv64-linux-gnu-gcc")
        .arg("--version")
        .output();
    let qemu_check = std::process::Command::new("qemu-riscv32")
        .arg("--version")
        .output();
    if gcc_check.is_err() || qemu_check.is_err() {
        if !quiet {
            println!("riscv64-linux-gnu-gcc or qemu-riscv32 is unavailable. Skipping RV32I verification.");
        }
        return Ok(false);
    }

    // 4. Write assembly and compile
    let cur_dir = std::env::current_dir()?;
    let input_name = Path::new(input_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("temp")
        .replace('.', "_");
    let s_path = cur_dir.join(format!("temp_mirtool_rv32_{}.s", input_name));
    let bin_path = cur_dir.join(format!("temp_mirtool_rv32_{}", input_name));

    std::fs::write(&s_path, full_asm)?;

    let mut compile_cmd = std::process::Command::new("riscv64-linux-gnu-gcc");
    compile_cmd
        .arg("-mabi=ilp32")
        .arg("-march=rv32im")
        .arg("-static")
        .arg("-nostdlib")
        .arg("-o")
        .arg(&bin_path)
        .arg(&s_path);

    let compile_output = compile_cmd.output();
    if !keep_temp {
        let _ = std::fs::remove_file(&s_path);
    }

    match compile_output {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !quiet {
                    println!("FAIL: RV32I compilation failed:\n{}", stderr);
                }
                return Ok(false);
            }
        }
        Err(err) => {
            if !quiet {
                println!("FAIL: Failed to run riscv64-linux-gnu-gcc: {}", err);
            }
            return Ok(false);
        }
    }

    // 5. Run under QEMU
    let run_output = std::process::Command::new("qemu-riscv32")
        .arg(&bin_path)
        .output();

    if !keep_temp {
        let _ = std::fs::remove_file(&bin_path);
    }

    let output = match run_output {
        Ok(o) => o,
        Err(err) => {
            if !quiet {
                println!("FAIL: Failed to run qemu-riscv32: {}", err);
            }
            return Ok(false);
        }
    };

    let exit_code = if let Some(code) = output.status.code() {
        code
    } else if let Some(sig) = output.status.signal() {
        128 + sig
    } else {
        255
    };

    // 6. Compare outcomes
    let is_pass = match expected {
        DiffOutcome::Success(expected_values) => {
            let expected_code = first_result_exit_code(&expected_values);
            let expected_exit_status = (expected_code & 0xff) as i32;
            let actual_exit_status = exit_code & 0xff;
            if actual_exit_status == expected_exit_status {
                if !quiet {
                    println!("PASS");
                }
                true
            } else {
                if !quiet {
                    println!(
                        "FAIL: Result mismatch. Expected exit code {} (masked: {}), got {}",
                        expected_code, expected_exit_status, actual_exit_status
                    );
                }
                false
            }
        }
        DiffOutcome::Trap(_) => {
            let actual_exit_status = exit_code & 0xff;
            // On RV32I QEMU, execution traps (ebreak or memory faults)
            // trigger a SIGSEGV (signal 11) or SIGTRAP (signal 5), returning exit status 139 or 133.
            if actual_exit_status == 139 || actual_exit_status == 133 {
                if !quiet {
                    println!("PASS");
                }
                true
            } else {
                if !quiet {
                    println!(
                        "FAIL: Trap mismatch. Expected exit status to match trap (139 or 133), got {}",
                        actual_exit_status
                    );
                }
                false
            }
        }
    };

    Ok(is_pass)
}

pub fn cmd_diff_all(keep_temp: bool, optimize: bool) -> Result<(), CliError> {
    let fixtures_dir = match find_fixtures_dir() {
        Some(dir) => dir,
        None => {
            return Err(CliError::Generic(
                "Failed to find fixtures directory".to_string(),
            ))
        }
    };

    let cc_available = std::process::Command::new("cc")
        .arg("--version")
        .output()
        .is_ok();

    let m2b_path = "/home/john/project/mir-preservation/git/mir-restored/m2b";
    let mir_bin_run_path = "/home/john/project/mir-preservation/git/mir-restored/mir-bin-run";
    let upstream_available =
        std::path::Path::new(m2b_path).exists() && std::path::Path::new(mir_bin_run_path).exists();

    let gcc_check = std::process::Command::new("riscv64-linux-gnu-gcc")
        .arg("--version")
        .output();
    let qemu_check = std::process::Command::new("qemu-riscv32")
        .arg("--version")
        .output();
    let rv32_available = gcc_check.is_ok() && qemu_check.is_ok();

    println!("=====================================================================");
    println!("           MIR-RETRODOC REGRESSION & DIFFERENTIAL TESTS              ");
    println!("=====================================================================");
    println!(
        "C Transpiler Diff (cc):   {}",
        if cc_available { "ENABLED" } else { "DISABLED" }
    );
    println!(
        "Upstream MIR Diff (m2b):  {}",
        if upstream_available {
            "ENABLED"
        } else {
            "DISABLED"
        }
    );
    println!(
        "RV32I QEMU Diff (gcc):    {}",
        if rv32_available {
            "ENABLED"
        } else {
            "DISABLED"
        }
    );
    println!("=====================================================================\n");

    let mut paths = Vec::new();
    for entry in std::fs::read_dir(fixtures_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("valid_") || name.starts_with("trap_") {
                    if name.ends_with(".mircap.txt") {
                        paths.push(path);
                    }
                }
            }
        }
    }
    paths.sort();

    let mut pass_count = 0;
    let mut fail_count = 0;
    let mut skip_count = 0;

    println!(
        "{:<40} | {:<12} | {:<12} | {:<12} | {:<12}",
        "Fixture Name", "Interpreter", "C Transpiler", "Upstream MIR", "RV32I QEMU"
    );
    println!(
        "{:-<40}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}",
        "", "", "", "", ""
    );

    for path in paths {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let path_str = path.to_string_lossy();
        let image = load_image(&path_str, None)?;
        let uses_float = module_uses_float(&image);

        // 1. Interpreter check
        let mut interp_status = "PASS";
        let runner = Runner::new(image, mirsem::ExecutionProfile::default());
        if runner.is_err() {
            interp_status = "FAIL";
        } else {
            let mut r = runner.unwrap();
            match r.run_entry_by_name("main", &[]) {
                Ok(_) | Err(mirsem::RunError::Trap(_)) => {}
                _ => {
                    interp_status = "FAIL";
                }
            }
        }

        // 2. C Transpiler check
        let mut c_status = "SKIP";
        let mut c_passed = true;
        if cc_available {
            match cmd_diff(&path_str, None, "main", keep_temp, optimize, true) {
                Ok(passed) => {
                    c_passed = passed;
                    c_status = if passed { "PASS" } else { "FAIL" };
                }
                Err(_) => {
                    c_passed = false;
                    c_status = "FAIL";
                }
            }
        }

        // 3. Upstream MIR check
        let mut upstream_status = "SKIP";
        let mut upstream_passed = true;
        if uses_float {
            upstream_status = "SKIP";
        } else if upstream_available {
            match cmd_diff_upstream(&path_str, None, "main", keep_temp, optimize, true) {
                Ok(passed) => {
                    upstream_passed = passed;
                    upstream_status = if passed { "PASS" } else { "FAIL" };
                }
                Err(_) => {
                    upstream_passed = false;
                    upstream_status = "FAIL";
                }
            }
        }

        // 4. RV32I check
        let mut rv32_status = "SKIP";
        let mut rv32_passed = true;
        if uses_float {
            rv32_status = "SKIP";
        } else if rv32_available {
            match cmd_diff_rv32i(&path_str, None, keep_temp, optimize, true) {
                Ok(passed) => {
                    rv32_passed = passed;
                    rv32_status = if passed { "PASS" } else { "FAIL" };
                }
                Err(_) => {
                    rv32_passed = false;
                    rv32_status = "FAIL";
                }
            }
        }

        let is_failed = interp_status == "FAIL" || !c_passed || !upstream_passed || !rv32_passed;
        if is_failed {
            fail_count += 1;
        } else {
            if c_status == "SKIP" && upstream_status == "SKIP" && rv32_status == "SKIP" {
                skip_count += 1;
            } else {
                pass_count += 1;
            }
        }

        println!(
            "{:<40} | {:<12} | {:<12} | {:<12} | {:<12}",
            name, interp_status, c_status, upstream_status, rv32_status
        );
    }

    println!("\n=====================================================================");
    println!(
        "Summary: {} Passed, {} Failed, {} Skipped",
        pass_count, fail_count, skip_count
    );
    println!("=====================================================================");

    if fail_count > 0 {
        return Err(CliError::Generic(format!("{} tests failed", fail_count)));
    }
    Ok(())
}

fn module_uses_float(image: &ModuleImage) -> bool {
    image
        .types
        .iter()
        .any(|ty| matches!(ty.kind, mircap::TypeKind::F32 | mircap::TypeKind::F64))
}

fn find_fixtures_dir() -> Option<std::path::PathBuf> {
    let mut path = std::env::current_dir().ok()?;
    loop {
        let fixtures = path.join("experiment/mircap/tests/fixtures");
        if fixtures.exists() && fixtures.is_dir() {
            return Some(fixtures);
        }
        if !path.pop() {
            break;
        }
    }
    None
}

pub fn cmd_growth(emit_json: bool, keep_temp: bool) -> Result<(), CliError> {
    let dir = Path::new("growth_fixtures");
    std::fs::create_dir_all(dir)?;

    let families = vec![
        ("straight-line arithmetic", "arithmetic"),
        ("branch-heavy code", "branch_heavy"),
        ("memory loop / summation", "memory_loop"),
        ("direct-call chain", "direct_call"),
    ];

    let sizes = vec![8, 16, 32, 64, 128];

    #[derive(Clone, Debug)]
    struct FamilyMeasurement {
        size: u32,
        instructions: u64,
        branches: u64,
        calls: u64,
        memory_reads: u64,
        memory_writes: u64,
        allocations: u64,
    }

    struct FamilyResult {
        name: &'static str,
        measurements: Vec<FamilyMeasurement>,
        classifications: BTreeMap<String, String>,
    }

    let mut results = Vec::new();

    for &(family_name, family_code) in &families {
        let mut measurements = Vec::new();

        for &size in &sizes {
            let content = match family_code {
                "arithmetic" => generate_straight_line_arithmetic(size),
                "branch_heavy" => generate_branch_heavy(size),
                "memory_loop" => generate_memory_loop_sum(size),
                "direct_call" => generate_direct_call_chain(size),
                _ => unreachable!(),
            };

            let file_name = format!("{}_{}.mircap.txt", family_code, size);
            let file_path = dir.join(&file_name);
            std::fs::write(&file_path, content)?;

            let path_str = file_path.to_string_lossy().to_string();

            let image = load_image(&path_str, Some("text"))?;
            image.validate().map_err(CliError::Validation)?;

            let mut runner = Runner::new(image.clone(), mirsem::ExecutionProfile::default())?;
            runner.run_entry_by_name("main", &[]).map_err(CliError::Run)?;
            let snapshot = runner.trace_snapshot();
            let observed = observed_total_cost_counts(&snapshot);

            if size == 8 || size == 16 {
                let diff_ok = cmd_diff(&path_str, Some("text"), "main", keep_temp, false, true)?;
                if !diff_ok {
                    return Err(CliError::Generic(format!(
                        "Differential check failed for family '{}' at size {}",
                        family_name, size
                    )));
                }
            }

            measurements.push(FamilyMeasurement {
                size,
                instructions: observed.instructions,
                branches: observed.branches,
                calls: observed.calls,
                memory_reads: observed.memory_reads,
                memory_writes: observed.memory_writes,
                allocations: observed.allocations,
            });
        }

        let mut classifications = BTreeMap::new();
        let x: Vec<f64> = sizes.iter().map(|&s| s as f64).collect();

        let y_insn: Vec<f64> = measurements.iter().map(|m| m.instructions as f64).collect();
        classifications.insert("instructions".to_string(), classify_growth(&x, &y_insn).to_string());

        let y_branch: Vec<f64> = measurements.iter().map(|m| m.branches as f64).collect();
        classifications.insert("branches".to_string(), classify_growth(&x, &y_branch).to_string());

        let y_call: Vec<f64> = measurements.iter().map(|m| m.calls as f64).collect();
        classifications.insert("calls".to_string(), classify_growth(&x, &y_call).to_string());

        let y_mem_read: Vec<f64> = measurements.iter().map(|m| m.memory_reads as f64).collect();
        classifications.insert("memory_reads".to_string(), classify_growth(&x, &y_mem_read).to_string());

        let y_mem_write: Vec<f64> = measurements.iter().map(|m| m.memory_writes as f64).collect();
        classifications.insert("memory_writes".to_string(), classify_growth(&x, &y_mem_write).to_string());

        let y_alloc: Vec<f64> = measurements.iter().map(|m| m.allocations as f64).collect();
        classifications.insert("allocations".to_string(), classify_growth(&x, &y_alloc).to_string());

        results.push(FamilyResult {
            name: family_name,
            measurements,
            classifications,
        });
    }

    if emit_json {
        let json_families: Vec<serde_json::Value> = results.iter().map(|r| {
            let json_measurements: Vec<serde_json::Value> = r.measurements.iter().map(|m| {
                json!({
                    "size": m.size,
                    "instructions": m.instructions,
                    "branches": m.branches,
                    "calls": m.calls,
                    "memory_reads": m.memory_reads,
                    "memory_writes": m.memory_writes,
                    "allocations": m.allocations,
                })
            }).collect();

            json!({
                "name": r.name,
                "sizes": sizes,
                "measurements": json_measurements,
                "classifications": r.classifications,
            })
        }).collect();

        let json_report = json!({
            "kind": "growth-analysis",
            "fixtures_directory": "growth_fixtures",
            "families": json_families
        });

        println!("{}", serde_json::to_string_pretty(&json_report).unwrap());
    } else {
        println!("\n=================================================================================");
        println!("Empirical Growth Analysis Report (v0)");
        println!("=================================================================================");
        println!("Generated fixtures stored in: ./growth_fixtures/");
        println!("=================================================================================\n");

        for r in &results {
            println!("Family: {}", r.name);
            println!("{:<8} | {:<10} | {:<8} | {:<8} | {:<10} | {:<10} | {:<8}", 
                "Size", "Insn", "Branch", "Call", "MemRead", "MemWrite", "Alloc");
            println!("{}", "-".repeat(81));
            for m in &r.measurements {
                println!("{:<8} | {:<10} | {:<8} | {:<8} | {:<10} | {:<10} | {:<8}",
                    m.size, m.instructions, m.branches, m.calls, m.memory_reads, m.memory_writes, m.allocations);
            }
            println!("{}", "-".repeat(81));
            println!("{:<8} | {:<10} | {:<8} | {:<8} | {:<10} | {:<10} | {:<8}",
                "Growth",
                r.classifications.get("instructions").unwrap(),
                r.classifications.get("branches").unwrap(),
                r.classifications.get("calls").unwrap(),
                r.classifications.get("memory_reads").unwrap(),
                r.classifications.get("memory_writes").unwrap(),
                r.classifications.get("allocations").unwrap()
            );
            println!("=================================================================================\n");
        }

        println!("=================================================================================");
        println!("Complexity Classification Summary");
        println!("=================================================================================");
        println!("{:<28} | {:<10} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8}",
            "Family", "Insn", "Branch", "Call", "MemRead", "MemWrite", "Alloc");
        println!("{}", "-".repeat(81));
        for r in &results {
            println!("{:<28} | {:<10} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8}",
                r.name,
                r.classifications.get("instructions").unwrap(),
                r.classifications.get("branches").unwrap(),
                r.classifications.get("calls").unwrap(),
                r.classifications.get("memory_reads").unwrap(),
                r.classifications.get("memory_writes").unwrap(),
                r.classifications.get("allocations").unwrap()
            );
        }
        println!("=================================================================================\n");
    }

    Ok(())
}

fn generate_straight_line_arithmetic(n: u32) -> String {
    let mut s = String::new();
    s.push_str("mircap mircap\nversion 0\nmodule 1 straight_line_arithmetic\ntype 1 i32\nsymbol 1 main function\n");
    let val_count = n + 2;
    let val_types = vec!["1"; val_count as usize];
    s.push_str(&format!("function 1 1 - 1 {} 0 {}\n", val_count, val_types.join(",")));
    s.push_str("func_block 1 1\n");
    
    let mut insn_ids = Vec::new();
    s.push_str("insn 1 const_i32 r:0 i:0\n");
    insn_ids.push("1".to_string());
    s.push_str("insn 2 const_i32 r:1 i:1\n");
    insn_ids.push("2".to_string());
    for i in 0..n {
        let insn_id = 3 + i;
        let dest_reg = 2 + i;
        let src_reg = if i == 0 { 0 } else { 2 + i - 1 };
        s.push_str(&format!("insn {} add_i32 r:{} v:{} v:1\n", insn_id, dest_reg, src_reg));
        insn_ids.push(insn_id.to_string());
    }
    let ret_insn_id = 3 + n;
    let ret_reg = if n == 0 { 0 } else { 2 + n - 1 };
    s.push_str(&format!("insn {} ret v:{}\n", ret_insn_id, ret_reg));
    insn_ids.push(ret_insn_id.to_string());
    
    s.push_str(&format!("block 1 1 {}\n", insn_ids.join(" ")));
    s
}

fn generate_branch_heavy(n: u32) -> String {
    let mut s = String::new();
    s.push_str("mircap mircap\nversion 0\nmodule 1 branch_heavy\ntype 1 i32\ntype 2 u32\nsymbol 1 main function\n");
    s.push_str("function 1 1 - 1 4 0 1,1,2,1\n");
    s.push_str("func_block 1 1\n");
    s.push_str("func_block 1 2\n");
    s.push_str("func_block 1 3\n");
    s.push_str("func_block 1 4\n");
    s.push_str("block 1 1 1 2 3 4\n");
    s.push_str("block 2 1 5 6\n");
    s.push_str("block 3 1 7 8\n");
    s.push_str("block 4 1 9\n");
    s.push_str("insn 1 const_i32 r:0 i:0\n");
    s.push_str(&format!("insn 2 const_i32 r:1 i:{}\n", n));
    s.push_str("insn 3 const_i32 r:3 i:1\n");
    s.push_str("insn 4 branch b:2\n");
    s.push_str("insn 5 lt_i32 r:2 v:0 v:1\n");
    s.push_str("insn 6 branch_if v:2 b:3 b:4\n");
    s.push_str("insn 7 add_i32 r:0 v:0 v:3\n");
    s.push_str("insn 8 branch b:2\n");
    s.push_str("insn 9 ret v:0\n");
    s
}

fn generate_memory_loop_sum(n: u32) -> String {
    let mut s = String::new();
    s.push_str("mircap mircap\nversion 0\nmodule 1 memory_loop_sum\ntype 1 i32\ntype 2 u32\ntype 3 addr32\nsymbol 1 main function\n");
    let mut val_types = vec!["3", "2", "3", "1", "1", "1", "1", "2", "1"];
    for _ in 0..n {
        val_types.push("2");
    }
    let val_count = val_types.len();
    s.push_str(&format!("function 1 1 - 1 {} 0 {}\n", val_count, val_types.join(",")));
    
    s.push_str("func_block 1 1\n");
    s.push_str("func_block 1 2\n");
    s.push_str("func_block 1 3\n");
    s.push_str("func_block 1 4\n");
    
    let mut init_insns = vec![
        format!("insn 1 alloc r:0 u:{} u:4", n * 4),
        format!("insn 2 const_u32 r:1 u:4"),
        format!("insn 3 copy r:2 v:0"),
    ];
    
    let mut insn_id = 4;
    for i in 0..n {
        init_insns.push(format!("insn {} const_u32 r:{} u:{}", insn_id, 9 + i, i));
        insn_id += 1;
        init_insns.push(format!("insn {} store_u32 v:2 v:{}", insn_id, 9 + i));
        insn_id += 1;
        if i < n - 1 {
            init_insns.push(format!("insn {} addr_add r:2 v:2 v:1", insn_id));
            insn_id += 1;
        }
    }
    
    let mut block1_ids: Vec<String> = (1..insn_id).map(|x| x.to_string()).collect();
    let setup_insns = vec![
        format!("insn {} const_i32 r:3 i:0", insn_id),
        format!("insn {} const_i32 r:4 i:{}", insn_id + 1, n),
        format!("insn {} const_i32 r:5 i:1", insn_id + 2),
        format!("insn {} const_i32 r:6 i:0", insn_id + 3),
        format!("insn {} copy r:2 v:0", insn_id + 4),
        format!("insn {} branch b:2", insn_id + 5),
    ];
    for id in insn_id..(insn_id + 6) {
        block1_ids.push(id.to_string());
    }
    insn_id += 6;
    
    let lt_insn_id = insn_id;
    let branch_if_insn_id = insn_id + 1;
    let ret_insn_id = insn_id + 2;
    let load_insn_id = insn_id + 3;
    let add_insn_id = insn_id + 4;
    let addr_add_insn_id = insn_id + 5;
    let loop_add_insn_id = insn_id + 6;
    let loop_branch_insn_id = insn_id + 7;
    
    for insn in &init_insns {
        s.push_str(insn);
        s.push_str("\n");
    }
    for insn in &setup_insns {
        s.push_str(insn);
        s.push_str("\n");
    }
    s.push_str(&format!("insn {} lt_i32 r:7 v:6 v:4\n", lt_insn_id));
    s.push_str(&format!("insn {} branch_if v:7 b:4 b:3\n", branch_if_insn_id));
    s.push_str(&format!("insn {} ret v:3\n", ret_insn_id));
    s.push_str(&format!("insn {} load_i32 r:8 v:2\n", load_insn_id));
    s.push_str(&format!("insn {} add_i32 r:3 v:3 v:8\n", add_insn_id));
    s.push_str(&format!("insn {} addr_add r:2 v:2 v:1\n", addr_add_insn_id));
    s.push_str(&format!("insn {} add_i32 r:6 v:6 v:5\n", loop_add_insn_id));
    s.push_str(&format!("insn {} branch b:2\n", loop_branch_insn_id));
    
    s.push_str(&format!("block 1 1 {}\n", block1_ids.join(" ")));
    s.push_str(&format!("block 2 1 {} {}\n", lt_insn_id, branch_if_insn_id));
    s.push_str(&format!("block 3 1 {}\n", ret_insn_id));
    s.push_str(&format!("block 4 1 {} {} {} {} {}\n", load_insn_id, add_insn_id, addr_add_insn_id, loop_add_insn_id, loop_branch_insn_id));
    
    s
}

fn generate_direct_call_chain(n: u32) -> String {
    let mut s = String::new();
    s.push_str("mircap mircap\nversion 0\n");
    s.push_str(&format!("module 1 direct_call_chain_{}\n", n));
    s.push_str("type 1 i32\n");
    
    s.push_str("symbol 1 main function\n");
    for i in 1..=n {
        s.push_str(&format!("symbol {} f{} function\n", i + 1, i));
    }
    
    for i in 1..=n {
        s.push_str(&format!("function {} 1 - 1 1 0 1\n", i));
    }
    s.push_str(&format!("function {} 1 - 1 1 0 1\n", n + 1));
    
    for i in 1..=(n + 1) {
        s.push_str(&format!("func_block {} {}\n", i, i));
    }
    
    for i in 1..=n {
        let insn_call = 2 * i - 1;
        let insn_ret = 2 * i;
        s.push_str(&format!("block {} {} {} {}\n", i, i, insn_call, insn_ret));
    }
    let last_const = 2 * n + 1;
    let last_ret = 2 * n + 2;
    s.push_str(&format!("block {} {} {} {}\n", n + 1, n + 1, last_const, last_ret));
    
    for i in 1..=n {
        let insn_call = 2 * i - 1;
        let insn_ret = 2 * i;
        s.push_str(&format!("insn {} call r:0 f:{}\n", insn_call, i + 1));
        s.push_str(&format!("insn {} ret v:0\n", insn_ret));
    }
    s.push_str(&format!("insn {} const_i32 r:0 i:42\n", last_const));
    s.push_str(&format!("insn {} ret v:0\n", last_ret));
    
    s
}

fn classify_growth(x: &[f64], y: &[f64]) -> &'static str {
    if y.iter().all(|&val| (val - y[0]).abs() < 1e-9) {
        return "constant";
    }
    
    if x.len() >= 2 {
        let a = (y[1] - y[0]) / (x[1] - x[0]);
        let b = y[0] - a * x[0];
        
        let mut is_linear = true;
        for i in 2..x.len() {
            let expected = a * x[i] + b;
            if (y[i] - expected).abs() > 1e-2 {
                is_linear = false;
                break;
            }
        }
        if is_linear && a.abs() > 1e-9 {
            return "linear";
        }
    }
    
    "unknown"
}
