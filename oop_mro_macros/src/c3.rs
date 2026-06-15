use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum C3Error {
    Cycle { class: usize },
    Inconsistent { class: usize },
}

impl C3Error {
    pub fn message(&self, names: &[String]) -> String {
        match *self {
            Self::Cycle { class } => {
                format!("inheritance cycle involving class `{}`", names[class])
            }
            Self::Inconsistent { class } => {
                format!(
                    "cannot compute a consistent C3 method resolution order for class `{}`",
                    names[class]
                )
            }
        }
    }
}

impl fmt::Display for C3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Cycle { class } => write!(f, "cycle at class index {class}"),
            Self::Inconsistent { class } => write!(f, "inconsistent class index {class}"),
        }
    }
}

pub fn linearize_all(bases: &[Vec<usize>]) -> Result<Vec<Vec<usize>>, C3Error> {
    let mut state = vec![VisitState::Unvisited; bases.len()];
    let mut mros = vec![Vec::new(); bases.len()];

    for index in 0..bases.len() {
        linearize(index, bases, &mut state, &mut mros)?;
    }

    Ok(mros)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Visiting,
    Done,
}

fn linearize(
    index: usize,
    bases: &[Vec<usize>],
    state: &mut [VisitState],
    mros: &mut [Vec<usize>],
) -> Result<Vec<usize>, C3Error> {
    match state[index] {
        VisitState::Done => return Ok(mros[index].clone()),
        VisitState::Visiting => return Err(C3Error::Cycle { class: index }),
        VisitState::Unvisited => {}
    }

    state[index] = VisitState::Visiting;

    let mut sequences = Vec::new();
    for &base in &bases[index] {
        sequences.push(linearize(base, bases, state, mros)?);
    }
    sequences.push(bases[index].clone());

    let mut result = vec![index];
    result.extend(merge(sequences).ok_or(C3Error::Inconsistent { class: index })?);

    state[index] = VisitState::Done;
    mros[index] = result.clone();
    Ok(result)
}

fn merge(mut sequences: Vec<Vec<usize>>) -> Option<Vec<usize>> {
    let mut result = Vec::new();

    loop {
        sequences.retain(|sequence| !sequence.is_empty());
        if sequences.is_empty() {
            return Some(result);
        }

        let mut candidate = None;
        'heads: for sequence in &sequences {
            let head = sequence[0];
            for other in &sequences {
                if other.iter().skip(1).any(|&item| item == head) {
                    continue 'heads;
                }
            }
            candidate = Some(head);
            break;
        }

        let candidate = candidate?;
        result.push(candidate);
        for sequence in &mut sequences {
            if sequence.first().copied() == Some(candidate) {
                sequence.remove(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_inheritance() {
        let bases = vec![vec![], vec![0], vec![1]];

        let mros = linearize_all(&bases).unwrap();

        assert_eq!(mros[0], vec![0]);
        assert_eq!(mros[1], vec![1, 0]);
        assert_eq!(mros[2], vec![2, 1, 0]);
    }

    #[test]
    fn diamond_inheritance() {
        let bases = vec![vec![], vec![0], vec![0], vec![1, 2]];

        let mros = linearize_all(&bases).unwrap();

        assert_eq!(mros[3], vec![3, 1, 2, 0]);
    }

    #[test]
    fn inconsistent_hierarchy_is_rejected() {
        let bases = vec![vec![], vec![], vec![0, 1], vec![1, 0], vec![2, 3]];

        let error = linearize_all(&bases).unwrap_err();

        assert_eq!(error, C3Error::Inconsistent { class: 4 });
    }

    #[test]
    fn cycles_are_rejected() {
        let bases = vec![vec![1], vec![0]];

        let error = linearize_all(&bases).unwrap_err();

        assert!(matches!(error, C3Error::Cycle { .. }));
    }
}
