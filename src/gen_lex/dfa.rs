use crate::gen_lex::utils::{epsilon_closure, move_states};
use crate::gen_lex::nfa::Nfa;
use std::collections::{HashMap, HashSet, VecDeque};

pub struct Dfa {
    #[allow(dead_code)]
    pub states: Vec<HashSet<usize>>, // Cada estado DFA es un conjunto de estados NFA
    pub transitions: HashMap<(usize, char), usize>,
    #[allow(dead_code)]
    pub accept_states: HashMap<usize, String>, // Mapea estado DFA -> Tipo de Token (ID, LET, etc.)
}

impl Dfa {
    pub fn from_nfa(nfa: &Nfa, alphabet: &Vec<char>, token_map: &Vec<(usize, String)>) -> Self {
        let mut states = Vec::new();
        let mut transitions = HashMap::new();
        let mut accept_states = HashMap::new();
        let mut queue = VecDeque::new();
        
        // El estado inicial del DFA es la clausura-ε del estado inicial del NFA
        let mut start_set = HashSet::new();
        start_set.insert(nfa.start);
        let start_closure = epsilon_closure(&start_set, nfa);
        
        states.push(start_closure.clone());
        queue.push_back(0); // Índice del estado procesando

        while let Some(u_idx) = queue.pop_front() {
            let u_set = states[u_idx].clone();
            
            // Verificar si este estado DFA contiene algún estado aceptador del NFA
            // IMPORTANTE: Buscar en el orden de token_map para respetar prioridad
            for (token_nfa_state, token_name) in token_map {
                if u_set.contains(token_nfa_state) && nfa.states[*token_nfa_state].is_accept {
                    // eprintln!("DFA state {} is accepting for token {} because NFA state {} is in {:?}", u_idx, token_name, token_nfa_state, u_set);
                    accept_states.insert(u_idx, token_name.clone());
                    break; // Tomar el primero en orden de prioridad
                }
            }

            for &symbol in alphabet {
                let move_set = move_states(&u_set, symbol, nfa);
                if move_set.is_empty() { continue; }
                
                let next_closure = epsilon_closure(&move_set, nfa);

                // Si el conjunto de estados no existe, lo agregamos como nuevo estado DFA
                let pos = states.iter().position(|s| s == &next_closure);
                let v_idx = match pos {
                    Some(idx) => idx,
                    None => {
                        states.push(next_closure.clone());
                        let new_idx = states.len() - 1;
                        queue.push_back(new_idx);
                        new_idx
                    }
                };
                transitions.insert((u_idx, symbol), v_idx);
            }
        }

        Dfa { states, transitions, accept_states }
    }
}