use crate::gen_lex::regex_ast::RegexAST;
use crate::gen_lex::utils::Edge;

#[derive(Debug, Clone)]
pub struct NfaState {
    pub edges: Vec<Edge>,
    pub is_accept: bool,
}

#[derive(Debug, Clone)]
pub struct Nfa {
    pub states: Vec<NfaState>,
    pub start: usize,
    #[allow(dead_code)]
    pub end: usize,
}

impl Nfa {
    pub fn build_from_ast(ast: &RegexAST, states: &mut Vec<NfaState>) -> (usize, usize) {
        match ast {
            RegexAST::Literal(c) => {
                let start = states.len();
                let end = start + 1;
                // Estado inicial con transición al estado final consumiendo 'c'
                states.push(NfaState { edges: vec![Edge::Symbol(*c, end)], is_accept: false });
                states.push(NfaState { edges: vec![], is_accept: false });
                (start, end)
            }
            
            RegexAST::Epsilon => {
                let start = states.len();
                let end = start + 1;
                states.push(NfaState { edges: vec![Edge::Epsilon(end)], is_accept: false });
                states.push(NfaState { edges: vec![], is_accept: false });
                (start, end)
            }

            RegexAST::Concat(left, right) => {
                let (l_start, l_end) = Self::build_from_ast(left, states);
                let (r_start, r_end) = Self::build_from_ast(right, states);
                // La salida de 'left' se conecta a la entrada de 'right' vía épsilon
                states[l_end].edges.push(Edge::Epsilon(r_start));
                (l_start, r_end)
            }

            RegexAST::Union(left, right) => {
                let new_start = states.len();
                states.push(NfaState { edges: vec![], is_accept: false }); // Placeholder
                
                let (l_start, l_end) = Self::build_from_ast(left, states);
                let (r_start, r_end) = Self::build_from_ast(right, states);
                
                let new_end = states.len();
                states.push(NfaState { edges: vec![], is_accept: false });

                // Conectar nuevo inicio a los inicios de left y right
                states[new_start].edges.push(Edge::Epsilon(l_start));
                states[new_start].edges.push(Edge::Epsilon(r_start));
                
                // Conectar finales de left y right al nuevo final
                states[l_end].edges.push(Edge::Epsilon(new_end));
                states[r_end].edges.push(Edge::Epsilon(new_end));
                
                (new_start, new_end)
            }

            RegexAST::Star(inner) => {
                let new_start = states.len();
                states.push(NfaState { edges: vec![], is_accept: false }); // Placeholder

                let (i_start, i_end) = Self::build_from_ast(inner, states);

                let new_end = states.len();
                states.push(NfaState { edges: vec![], is_accept: false });

                // 1. Del nuevo inicio al inicio del interior (bucle)
                states[new_start].edges.push(Edge::Epsilon(i_start));
                // 2. Del nuevo inicio al nuevo fin (cero repeticiones)
                states[new_start].edges.push(Edge::Epsilon(new_end));
                // 3. Del fin del interior de vuelta al inicio (repetir)
                states[i_end].edges.push(Edge::Epsilon(i_start));
                // 4. Del fin del interior al nuevo fin (salir del bucle)
                states[i_end].edges.push(Edge::Epsilon(new_end));

                (new_start, new_end)
            }
        }
    }
}