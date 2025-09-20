use pest::error::Error;
use pest::iterators::Pairs;
use pest::Parser;

#[derive(Parser)]
#[grammar = "parsers/grammar.pest"]
struct Locust;

pub fn parse_lines(
    lines: &str,
) -> Result<Pairs<'_, crate::parsers::locust::Rule>, Error<crate::parsers::locust::Rule>> {
    Locust::parse(Rule::EXP, lines)
}

#[cfg(test)]
mod tests {
    use super::parse_lines;
    use super::Pairs;
    use super::Rule;

    fn _compose_pair_str(
        output: &mut String,
        lv: i32,
        pairs: Pairs<crate::parsers::locust::Rule>,
    ) {
        for pair in pairs {
            let value = pair.as_str().trim();
            if value.is_empty() {
                continue;
            }

            let rule = pair.as_rule();
            let mut i = 0;
            while i < lv {
                output.push_str("--");
                i += 1;
            }

            output.push_str(&format!("[{:?}]", rule));
            if rule == Rule::CMD || rule == Rule::TEST {
                output.push_str(&format!(" {}", value));
            }

            let pairs_new = pair.into_inner();
            _compose_pair_str(output, lv + 1, pairs_new);
        }
    }

    fn _parse_exp(lines: &str) -> String {
        let mut output = String::new();
        match parse_lines(lines) {
            Ok(x) => {
                _compose_pair_str(&mut output, 0, x);
            }
            Err(e) => {
                println!("parse error: {:?}", e);
                unreachable!();
            }
        }
        output
    }

    #[test]
    fn test_locust_parse_lines_exp_if() {
        let lines = include_str!("../../tests/locusts/if-001.sh");
        let expected = "\
            [EXP]\
            --[CMD] echo begin if exp\
            --[EXP_IF]\
            ----[IF_IF_BR]\
            ------[IF_HEAD]\
            --------[TEST] echo \"foo 中文\" | grep -qi foo\
            ------[EXP_BODY]\
            --------[CMD] echo \'we found it\'\
            ----[IF_ELSE_BR]\
            ------[KW_ELSE]\
            ------[EXP_BODY]\
            --------[CMD] echo not found\
            --[CMD] echo \"after fi\"\
            --[CMD] echo the end";
        let output = _parse_exp(lines);
        assert_eq!(output, expected);

        let lines = include_str!("../../tests/locusts/if-002.sh");
        let output = _parse_exp(lines);
        assert_eq!(output, expected);

        let lines = include_str!("../../tests/locusts/if-003.sh");
        let output = _parse_exp(lines);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_locust_parse_lines_exp_if_nest() {
        let lines = include_str!("../../tests/locusts/if-nest-001.sh");
        let expected = "\
            [EXP]\
            --[EXP_IF]\
            ----[IF_IF_BR]\
            ------[IF_HEAD]\
            --------[TEST] test 001\
            ------[EXP_BODY]\
            --------[EXP_IF]\
            ----------[IF_IF_BR]\
            ------------[IF_HEAD]\
            --------------[TEST] test 002\
            ------------[EXP_BODY]\
            --------------[CMD] cmd a\
            --------------[EXP_IF]\
            ----------------[IF_IF_BR]\
            ------------------[IF_HEAD]\
            --------------------[TEST] test 003\
            ------------------[EXP_BODY]\
            --------------------[CMD] cmd b\
            ----------[IF_ELSE_BR]\
            ------------[KW_ELSE]\
            ------------[EXP_BODY]\
            --------------[CMD] cmd c\
            --------[CMD] cmd d\
            ----[IF_ELSE_BR]\
            ------[KW_ELSE]\
            ------[EXP_BODY]\
            --------[EXP_IF]\
            ----------[IF_IF_BR]\
            ------------[IF_HEAD]\
            --------------[TEST] test 004\
            ------------[EXP_BODY]\
            --------------[CMD] cmd e\
            ----------[IF_ELSE_BR]\
            ------------[KW_ELSE]\
            ------------[EXP_BODY]\
            --------------[CMD] cmd f\
            --------[CMD] cmd g";
        let output = _parse_exp(lines);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_locust_parse_lines_exp_if_nest_2() {
        let lines = include_str!("../../tests/locusts/if-nest-002.sh");
        let expected = "\
            [EXP]\
            --[EXP_IF]\
            ----[IF_IF_BR]\
            ------[IF_HEAD]\
            --------[TEST] test 001\
            ------[EXP_BODY]\
            --------[CMD] cmd a\
            ----[IF_ELSE_BR]\
            ------[KW_ELSE]\
            ------[EXP_BODY]\
            --------[CMD] cmd b";
        let output = _parse_exp(lines);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_locust_parse_lines_exp_else_if() {
        let lines = include_str!("../../tests/locusts/if-else-if-001.sh");
        let expected = "\
            [EXP]\
            --[EXP_IF]\
            ----[IF_IF_BR]\
            ------[IF_HEAD]\
            --------[TEST] test 1\
            ------[EXP_BODY]\
            --------[CMD] cmd a\
            ----[IF_ELSEIF_BR]\
            ------[IF_ELSEIF_HEAD]\
            --------[TEST] test 2\
            ------[EXP_BODY]\
            --------[CMD] cmd b\
            ----[IF_ELSEIF_BR]\
            ------[IF_ELSEIF_HEAD]\
            --------[TEST] test 3\
            ------[EXP_BODY]\
            --------[CMD] cmd c\
            ----[IF_ELSE_BR]\
            ------[KW_ELSE]\
            ------[EXP_BODY]\
            --------[CMD] cmd d";
        let output = _parse_exp(lines);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_locust_parse_lines_exp_while() {
        let lines = include_str!("../../tests/locusts/while-001.sh");
        let expected = "\
            [EXP]\
            --[CMD] echo while begins\
            --[EXP_WHILE]\
            ----[WHILE_HEAD]\
            ------[TEST] cat /tmp/foo.txt | grep -qi foo\
            ----[EXP_BODY]\
            ------[CMD] sleep 1\
            ------[CMD] echo foo is good in while\
            --[CMD] echo the end";
        let output = _parse_exp(lines);
        assert_eq!(output, expected);

        let lines = include_str!("../../tests/locusts/while-002.sh");
        let output = _parse_exp(lines);
        assert_eq!(output, expected);

        let lines = include_str!("../../tests/locusts/while-003.sh");
        let output = _parse_exp(lines);
        assert_eq!(output, expected);

        let lines = include_str!("../../tests/locusts/while-004.sh");
        let output = _parse_exp(lines);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_locust_parse_lines_exp_if_while_001() {
        let lines = include_str!("../../tests/locusts/if-while-001.sh");
        let expected = "\
            [EXP]\
            --[EXP_IF]\
            ----[IF_IF_BR]\
            ------[IF_HEAD]\
            --------[TEST] test 01\
            ------[EXP_BODY]\
            --------[CMD] cmd a\
            --------[EXP_WHILE]\
            ----------[WHILE_HEAD]\
            ------------[TEST] test 02\
            ----------[EXP_BODY]\
            ------------[CMD] cmd b\
            --------[CMD] cmd c";
        let output = _parse_exp(lines);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_locust_parse_lines_exp_while_if_001() {
        let lines = include_str!("../../tests/locusts/while-if-001.sh");
        let expected = "\
            [EXP]\
            --[EXP_WHILE]\
            ----[WHILE_HEAD]\
            ------[TEST] test 01\
            ----[EXP_BODY]\
            ------[CMD] cmd a\
            ------[EXP_IF]\
            --------[IF_IF_BR]\
            ----------[IF_HEAD]\
            ------------[TEST] test 02\
            ----------[EXP_BODY]\
            ------------[CMD] cmd b\
            ------[CMD] cmd c";
        let output = _parse_exp(lines);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_locust_parse_lines_exp_while_if_more_001() {
        let lines = include_str!("../../tests/locusts/while-if-more-001.sh");
        let expected = "\
            [EXP]\
            --[EXP_WHILE]\
            ----[WHILE_HEAD]\
            ------[TEST] test 01\
            ----[EXP_BODY]\
            ------[EXP_IF]\
            --------[IF_IF_BR]\
            ----------[IF_HEAD]\
            ------------[TEST] test 02\
            ----------[EXP_BODY]\
            ------------[EXP_WHILE]\
            --------------[WHILE_HEAD]\
            ----------------[TEST] test 03\
            --------------[EXP_BODY]\
            ----------------[EXP_IF]\
            ------------------[IF_IF_BR]\
            --------------------[IF_HEAD]\
            ----------------------[TEST] test 04\
            --------------------[EXP_BODY]\
            ----------------------[CMD] cmd a";
        let output = _parse_exp(lines);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_locust_parse_lines_exp_while_if_more_002() {
        let lines = include_str!("../../tests/locusts/while-if-more-002.sh");
        let expected = "\
            [EXP]\
            --[EXP_IF]\
            ----[IF_IF_BR]\
            ------[IF_HEAD]\
            --------[TEST] test 01\
            ------[EXP_BODY]\
            --------[EXP_WHILE]\
            ----------[WHILE_HEAD]\
            ------------[TEST] test 02\
            ----------[EXP_BODY]\
            ------------[EXP_IF]\
            --------------[IF_IF_BR]\
            ----------------[IF_HEAD]\
            ------------------[TEST] test 03\
            ----------------[EXP_BODY]\
            ------------------[CMD] cmd a\
            ------------------[EXP_WHILE]\
            --------------------[WHILE_HEAD]\
            ----------------------[TEST] test 04\
            --------------------[EXP_BODY]\
            ----------------------[CMD] cmd b\
            ------------[CMD] cmd c\
            --------[CMD] cmd d";
        let output = _parse_exp(lines);
        assert_eq!(output, expected);
    }
}
