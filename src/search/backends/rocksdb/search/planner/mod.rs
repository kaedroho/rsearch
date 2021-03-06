pub mod boolean_query;
pub mod score_function;

use search::Query;

use super::super::RocksDBReader;
use self::boolean_query::{BooleanQueryOp, BooleanQueryBuilder, plan_boolean_query};
use self::score_function::{ScoreFunctionOp, plan_score_function};

#[derive(Debug)]
pub struct SearchPlan {
    pub boolean_query: Vec<BooleanQueryOp>,
    pub boolean_query_is_negated: bool,
    pub score_function: Vec<ScoreFunctionOp>,
}

impl SearchPlan {
    pub fn new() -> SearchPlan {
        SearchPlan {
            boolean_query: Vec::new(),
            boolean_query_is_negated: false,
            score_function: Vec::new(),
        }
    }
}

pub fn plan_query(index_reader: &RocksDBReader, query: &Query, score: bool) -> SearchPlan {
    let mut plan = SearchPlan::new();

    // Plan boolean query
    let mut builder = BooleanQueryBuilder::new();
    plan_boolean_query(index_reader, &mut builder, query);

    // Add operations to exclude deleted documents to boolean query
    builder.push_deletion_list();
    builder.andnot_combinator();

    let (boolean_query, boolean_query_is_negated) = builder.build();
    plan.boolean_query = boolean_query;
    plan.boolean_query_is_negated = boolean_query_is_negated;

    // Plan score function
    if score {
        plan_score_function(index_reader, &mut plan.score_function, query);
    } else {
        plan.score_function.push(ScoreFunctionOp::Literal(0.0f32));
    }

    plan
}
