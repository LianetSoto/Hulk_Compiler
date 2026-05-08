#[derive(Debug, Clone)]
pub enum RegexAST {
    /// Un carácter literal exacto, ej: 'a', '1', '+'
    Literal(char),
    /// Concatenación de dos expresiones (AB)
    Concat(Box<RegexAST>, Box<RegexAST>),
    /// Unión de dos expresiones (A|B)
    Union(Box<RegexAST>, Box<RegexAST>),
    /// Clausura de Kleene (A*) - cero o más repeticiones
    Star(Box<RegexAST>),
    /// La cadena vacía (épsilon)
    Epsilon,
}

pub fn insert_explicit_concat(regex: &str) -> String {
    // Solo escapar paréntesis si el patrón COMPLETO es un paréntesis literal
    let regex = if regex == "(" {
        "\\(".to_string()
    } else if regex == ")" {
        "\\)".to_string()
    } else {
        regex.to_string()
    };
    
    let mut result = String::new();
    let chars: Vec<char> = regex.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let c1 = chars[i];
        
        // Manejar caracteres escapados
        if c1 == '\\' && i + 1 < chars.len() {
            result.push(c1);
            result.push(chars[i + 1]);
            i += 2;
            continue;
        }
        
        result.push(c1);
        i += 1;
        
        if i < chars.len() {
            let c2 = chars[i];
            // No insertar concatenación si el siguiente es un escape
            if c2 == '\\' {
                continue;
            }
            
            let c1_is_concat_left = !matches!(c1, '|' | '(');
            let c2_is_concat_right = !matches!(c2, '|' | ')' | '*' | '?' | '.');

            if c1_is_concat_left && c2_is_concat_right {
                result.push('.');
            }
        }
    }
    result
}

pub struct RegexParser {
    tokens: Vec<char>,
    pos: usize,
}

impl RegexParser {
    pub fn new(regex: &str) -> Self {
        let preprocessed = insert_explicit_concat(regex);
        RegexParser {
            tokens: preprocessed.chars().collect(),
            pos: 0,
        }
    }

    pub fn parse(&mut self) -> RegexAST {
        self.parse_union()
    }

    fn parse_union(&mut self) -> RegexAST {
        let mut left = self.parse_concat();
        while self.pos < self.tokens.len() && self.tokens[self.pos] == '|' {
            self.pos += 1; // Consumir el '|'
            let right = self.parse_concat();
            left = RegexAST::Union(Box::new(left), Box::new(right));
        }
        left
    }

    fn parse_concat(&mut self) -> RegexAST {
        let mut left = self.parse_star();

        // Mientras el siguiente carácter sea un punto de concatenación
        while self.pos < self.tokens.len() && self.tokens[self.pos] == '.' {
            self.pos += 1; // Consumir el '.'
            let right = self.parse_star();
            left = RegexAST::Concat(Box::new(left), Box::new(right));
        }
        left
    }

    // 3. parse_star: Maneja la clausura de Kleene '*' y opcional '?' (Precedencia más alta que concat)
    fn parse_star(&mut self) -> RegexAST {
        let mut node = self.parse_primary();
        while self.pos < self.tokens.len() {
            if self.tokens[self.pos] == '*' {
                self.pos += 1;
                node = RegexAST::Star(Box::new(node));
            } else if self.tokens[self.pos] == '?' {
                self.pos += 1;
                node = RegexAST::Union(Box::new(node), Box::new(RegexAST::Epsilon));
            } else {
                break;
            }
        }
        node
    }

    fn parse_primary(&mut self) -> RegexAST {
        match self.tokens.get(self.pos) {
            Some('\\') if self.pos + 1 < self.tokens.len() => {
                // Carácter escapado
                self.pos += 1; // Consumir backslash
                let escaped_char = self.tokens[self.pos];
                self.pos += 1; // Consumir el carácter
                RegexAST::Literal(escaped_char)
            }
            Some('(') => {
                self.pos += 1; // Consumir '('
                let node = self.parse_union(); // Volver a la precedencia más baja dentro
                if self.tokens.get(self.pos) == Some(&')') {
                    self.pos += 1; // Consumir ')'
                } else {
                    panic!("Error sintáctico: se esperaba ')'");
                }
                node
            }
            Some(c) if !['|', '*', '.', ')', '\\'].contains(c) => {
                let char_val = *c;
                self.pos += 1;
                RegexAST::Literal(char_val)
            }
            _ => {
                // Si no hay más tokens o es un carácter inesperado, asumimos Epsilon
                RegexAST::Epsilon
            }
        }
    }

   
}