//! Tests for semantic analysis

#[cfg(test)]
mod tests {
    use super::super::analyzer::SemanticAnalyzer;
    use crate::lang::lexer::Lexer;
    use crate::lang::parser::Parser;
    
    #[test]
    fn test_analyze_simple_function() {
        let source = "fn add(x : int, y : int) : int { return x + y; }";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();
        
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(&program);
        
        assert!(result.is_ok(), "Analysis should succeed");
    }
    
    #[test]
    fn test_type_mismatch_error() {
        let source = "fn test() : int { return true; }";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();
        
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(&program);
        
        assert!(result.is_err(), "Analysis should fail due to type mismatch");
        let errors = analyzer.get_errors();
        assert!(!errors.is_empty(), "Should have type errors");
    }

    #[test]
    fn test_undefined_variable_error() {
        let source = "fn test() : int { return x; }";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();
        
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(&program);
        
        assert!(result.is_err(), "Analysis should fail due to undefined variable");
        let errors = analyzer.get_errors();
        assert!(!errors.is_empty(), "Should have undefined variable error");
    }

    #[test]
    fn test_function_call_type_checking() {
        let source = r#"
            fn add(x : int, y : int) : int { return x + y; }
            fn test() : int { return add(1, 2); }
        "#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();
        
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(&program);
        
        assert!(result.is_ok(), "Analysis should succeed");
    }

    #[test]
    fn test_function_call_argument_mismatch() {
        let source = r#"
            fn add(x : int, y : int) : int { return x + y; }
            fn test() : int { return add(1); }
        "#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();
        
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(&program);
        
        assert!(result.is_err(), "Analysis should fail due to argument count mismatch");
    }

    #[test]
    fn test_variable_declaration_with_type_inference() {
        let source = r#"
            fn test() : int {
                let x = 42;
                return x;
            }
        "#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();
        
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(&program);
        
        assert!(result.is_ok(), "Analysis should succeed with type inference");
    }

    #[test]
    fn test_array_type_checking() {
        let source = r#"
            fn test() : [int] {
                let arr = [1, 2, 3];
                return arr;
            }
        "#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();
        
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(&program);
        
        assert!(result.is_ok(), "Analysis should succeed with array types");
    }

    #[test]
    fn test_control_flow_analysis() {
        let source = r#"
            fn test(x : int) : int {
                if x > 0 {
                    return x;
                } else {
                    return -x;
                }
            }
        "#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();
        
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(&program);
        
        assert!(result.is_ok(), "Analysis should succeed with control flow");
    }
}
