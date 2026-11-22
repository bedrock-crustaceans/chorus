use crate::bedrock::overworld::surface::info_noise;
use crate::shared::rng::mersenne::MersenneTwister;
use molangx::catalog::{Arity, MathCatalog, MathDecl, QueryAdmission, QueryCatalog, QuerySetMask, Side};
use molangx::compile::{CompileFailure, CompileOptions, CompileOutcome, Expr, compile};
use molangx::diag::{DiagCode, Severity};
use molangx::rng::rand_core::{Infallible, TryRng, utils};
use molangx::stdlib::{self, query};
use molangx::version::MolangVersion;
use molangx::vm::{EvalCx, EvalLimits, Host as MolangHost, HostAccess, NoContext, NullSink, QueryCx, QueryResult, QueryTable, Subjects, Temps, Value, VariableName, VariableStore};
use std::collections::BTreeMap;
use std::sync::LazyLock;

#[derive(Clone, Copy, Default)]
pub struct Variables {
    pub origin: [f32; 3],
    pub world: [f32; 3],
}

pub trait Host {
    fn heightmap(&mut self, x: i32, z: i32) -> i32;
    fn above_top_solid(&mut self, x: i32, z: i32) -> i32;
}

struct ScatterHost;

impl MolangHost for ScatterHost {
    type ActorRef = ();
    type ItemRef = ();
    type BlockRef = ();
    type Access<'w> = dyn Host + 'w;
}

impl HostAccess<ScatterHost> for dyn Host + '_ {
    fn resolve_actor(&self, _from: &Subjects<ScatterHost>, _actor: ()) -> Option<()> {
        None
    }
}

static CATALOG: LazyLock<QueryCatalog> = LazyLock::new(|| stdlib::queries(Side::Server).clone());

static MATH: LazyLock<MathCatalog> = LazyLock::new(|| {
    MathCatalog::new([MathDecl::volatile("math.random", Arity::exactly(2), |random, arguments| {
        let low = arguments[0];
        let high = arguments[1];
        let sample = MersenneTwister::float_from_word(random.next_u32());
        low + (high - low) * sample
    })
    .expect("math.random takes two arguments")])
    .expect("the host math catalogue is valid")
});

static QUERIES: LazyLock<QueryTable<ScatterHost>> = LazyLock::new(|| {
    let mut table = QueryTable::new(&CATALOG);
    table
        .set(query::HEIGHTMAP, |cx: &mut QueryCx<'_, '_, ScatterHost>| -> QueryResult<ScatterHost> {
            let x = cx.arg_f32(0).unwrap_or(0.0).round() as i32;
            let z = cx.arg_f32(1).unwrap_or(0.0).round() as i32;
            Ok(Value::Float(cx.host().heightmap(x, z) as f32))
        })
        .expect("query.heightmap is in the catalogue");
    table
        .set(query::ABOVE_TOP_SOLID, |cx: &mut QueryCx<'_, '_, ScatterHost>| -> QueryResult<ScatterHost> {
            let x = cx.arg_f32(0).unwrap_or(0.0).round() as i32;
            let z = cx.arg_f32(1).unwrap_or(0.0).round() as i32;
            Ok(Value::Float(cx.host().above_top_solid(x, z) as f32))
        })
        .expect("query.above_top_solid is in the catalogue");
    table
        .set(query::NOISE, |cx: &mut QueryCx<'_, '_, ScatterHost>| -> QueryResult<ScatterHost> {
            let x = cx.arg_f32(0).unwrap_or(0.0);
            let z = cx.arg_f32(1).unwrap_or(0.0);
            Ok(Value::Float(info_noise(x, z)))
        })
        .expect("query.noise is in the catalogue");
    table
});

fn options() -> CompileOptions {
    CompileOptions {
        admission: QueryAdmission::Sets(QuerySetMask::DEFAULT | QuerySetMask::WORLD_GEN),
        math: Some(MATH.clone()),
        ..CompileOptions::new(CATALOG.clone(), MolangVersion::LATEST)
    }
}

#[derive(Debug)]
pub enum Expression {
    Number(f32),
    Unsupported(String),
    Compiled(CompiledExpression),
}

#[derive(Debug)]
pub struct CompiledExpression {
    expr: Option<Expr>,
    unsupported: Option<String>,
}

impl Expression {
    pub fn parse(text: &str) -> Self {
        let compiled = compile(text, &options());
        let mut unsupported = match compiled.outcome() {
            CompileOutcome::Success(_) => None,
            CompileOutcome::Rejected(_) => compiled.diagnostics().iter().find(|diagnostic| diagnostic.severity() == Severity::Error).map(|diagnostic| {
                if diagnostic.code() == DiagCode::UnknownQuery {
                    let span = diagnostic.span();
                    text.get(span.start as usize..span.end as usize).unwrap_or(text).to_owned()
                } else {
                    diagnostic.message().into_owned()
                }
            }),
            _ => Some(format!("{:?}", compiled.failure().unwrap_or(CompileFailure::Rejected))),
        };
        let expr = compiled.expr_or_zero().cloned();
        if unsupported.is_none() {
            unsupported = expr.as_ref().and_then(|expr| expr.queries().find(|name| QUERIES.is_stub(name).unwrap_or(false))).map(str::to_owned);
        }
        Self::Compiled(CompiledExpression { expr, unsupported })
    }

    pub fn uses_random(&self) -> bool {
        match self {
            Self::Number(_) | Self::Unsupported(_) => false,
            Self::Compiled(compiled) => compiled.expr.as_ref().is_some_and(|expr| expr.has_side_effects(true)),
        }
    }

    pub fn constant(&self) -> Option<f32> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Unsupported(_) | Self::Compiled(_) => None,
        }
    }

    pub fn collect_unsupported(&self, into: &mut BTreeMap<String, usize>) {
        match self {
            Self::Number(_) => {}
            Self::Unsupported(name) => *into.entry(name.clone()).or_default() += 1,
            Self::Compiled(compiled) => {
                if let Some(name) = &compiled.unsupported {
                    *into.entry(name.clone()).or_default() += 1;
                }
            }
        }
    }

    pub fn evaluate(&self, variables: &Variables, host: &mut dyn Host, random: &mut MersenneTwister) -> f32 {
        match self {
            Self::Number(value) => *value,
            Self::Unsupported(_) => 0.0,
            Self::Compiled(compiled) => compiled.evaluate(variables, host, random),
        }
    }
}

impl CompiledExpression {
    fn evaluate(&self, variables: &Variables, host: &mut dyn Host, random: &mut MersenneTwister) -> f32 {
        let Some(expr) = &self.expr else { return 0.0 };
        let mut bindings = Bindings::new(variables);
        let mut random = ChunkRandom(random);
        let mut sink = NullSink;
        let mut cx = EvalCx {
            subjects: Subjects::none(),
            host,
            variables: &mut bindings,
            context: &NoContext,
            queries: Some(&QUERIES),
            rng: &mut random,
            sink: &mut sink,
            limits: EvalLimits::DEFAULT,
            temps: Temps::PerEvaluation,
        };
        expr.eval_f32(&mut cx)
    }
}

struct Bindings {
    values: [Value<ScatterHost>; 7],
}

const NAMES: [VariableName; 7] = [
    VariableName::new("worldx"),
    VariableName::new("worldy"),
    VariableName::new("worldz"),
    VariableName::new("originx"),
    VariableName::new("originy"),
    VariableName::new("originz"),
    VariableName::new("is_legacy"),
];

impl Bindings {
    fn new(variables: &Variables) -> Self {
        Self {
            values: [
                Value::Float(variables.world[0]),
                Value::Float(variables.world[1]),
                Value::Float(variables.world[2]),
                Value::Float(variables.origin[0]),
                Value::Float(variables.origin[1]),
                Value::Float(variables.origin[2]),
                Value::Float(0.0),
            ],
        }
    }
}

impl VariableStore<ScatterHost> for Bindings {
    fn get(&self, _actor: (), _name: VariableName) -> Option<&Value<ScatterHost>> {
        None
    }

    fn set(&mut self, _actor: (), _name: VariableName, _value: Value<ScatterHost>) {}

    fn get_public(&self, _actor: (), _name: VariableName) -> Option<&Value<ScatterHost>> {
        None
    }

    fn get_local(&self, name: VariableName) -> Option<&Value<ScatterHost>> {
        NAMES.iter().position(|known| *known == name).map(|index| &self.values[index])
    }
}

struct ChunkRandom<'a>(&'a mut MersenneTwister);

impl TryRng for ChunkRandom<'_> {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        Ok(self.0.next_u32())
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        utils::next_u64_via_u32(self)
    }

    fn try_fill_bytes(&mut self, destination: &mut [u8]) -> Result<(), Infallible> {
        utils::fill_bytes_via_next_word(destination, || self.try_next_u32())
    }
}
