// via: https://github.com/pest-parser/book/blob/b6a42eb7/examples/calculator/src/main.rs
use std::num::Wrapping as W;

use pest::Parser;
use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::{Assoc, Op, PrattParser};

#[derive(Parser)]
#[grammar = "calculator/grammar.pest"]
struct Calculator;

lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use Rule::*;
        use Assoc::*;

        PrattParser::new()
            .op(Op::infix(add, Left) | Op::infix(subtract, Left))
            .op(Op::infix(multiply, Left) | Op::infix(divide, Left))
            .op(Op::infix(power, Right))
    };
}

pub fn eval_int(expression: Pairs<Rule>) -> i64 {
    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::num => primary.as_str().parse::<i64>().unwrap(),
            Rule::expr => eval_int(primary.into_inner()),
            _ => unreachable!(),
        })
        .map_infix(|lhs: i64, op: Pair<Rule>, rhs: i64| match op.as_rule() {
            Rule::add => (W(lhs) + W(rhs)).0,
            Rule::subtract => (W(lhs) - W(rhs)).0,
            Rule::multiply => (W(lhs) * W(rhs)).0,
            Rule::divide => {
                if rhs == 0 {
                    (lhs as f64 / 0.0) as i64
                } else {
                    (W(lhs) / W(rhs)).0
                }
            }
            Rule::power => lhs.pow(rhs as u32),
            _ => unreachable!(),
        })
        .parse(expression)
}

pub fn eval_float(expression: Pairs<Rule>) -> f64 {
    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::num => primary.as_str().parse::<f64>().unwrap(),
            Rule::expr => eval_float(primary.into_inner()),
            _ => unreachable!(),
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::add => lhs + rhs,
            Rule::subtract => lhs - rhs,
            Rule::multiply => lhs * rhs,
            Rule::divide => lhs / rhs,
            Rule::power => lhs.powf(rhs),
            _ => unreachable!(),
        })
        .parse(expression)
}

pub fn calculate(line: &str) -> Result<pest::iterators::Pairs<'_, Rule>, pest::error::Error<Rule>> {
    Calculator::parse(Rule::calculation, line)
}
