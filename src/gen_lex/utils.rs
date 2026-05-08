use std::collections::HashSet;
use crate::gen_lex::nfa::Nfa;

// Algoritmo 3.11 del libro: Calcula la clausura épsilon
pub fn epsilon_closure(states: &HashSet<usize>, nfa: &Nfa) -> HashSet<usize> {
    let mut closure = states.clone();
    let mut stack: Vec<usize> = states.iter().cloned().collect();

    while let Some(u) = stack.pop() {
        for edge in &nfa.states[u].edges {
            if let &Edge::Epsilon(v) = edge {
                if closure.insert(v) {
                    stack.push(v);
                }
            }
        }
    }
    closure
}

// Algoritmo: Encuentra estados alcanzables con un carácter
pub fn move_states(states: &HashSet<usize>, symbol: char, nfa: &Nfa) -> HashSet<usize> {
    let mut result = HashSet::new();
    for &u in states {
        for edge in &nfa.states[u].edges {
            if let &Edge::Symbol(c, v) = edge {
                if c == symbol {
                    result.insert(v);
                }
            }
        }
    }
    result
}

#[derive(Debug, Clone)]
pub enum Edge {
    /// Representa una transición que consume un carácter.
    /// Contiene el carácter esperado y el índice del estado destino.
    Symbol(char, usize), 
    
    /// Representa una transición épsilon (vacía).
    /// Contiene solo el índice del estado destino.
    Epsilon(usize),
}