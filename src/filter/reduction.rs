use super::*;

// TODO: add fuzzy tests
// further reduced by assert-rs/predicates-rs or cfallin/boolean_expression ?
// but that might not be practical in most normal cases (i.e. no duplicates)
pub fn reduce(mut chain: Chain) -> Chain {
    while !chain.links.is_empty() {
        let mut recur = false;
        let mut links = Vec::<Link>::with_capacity(chain.links.len());
        let iter = chain.links.into_iter();

        iter.for_each(|link| {
            match &link.filter {
                Filter::Anything => match (&link.joint, !link.negated) {
                    (Joint::And, true) | (Joint::Or, false) | (Joint::Xor, false) => return,
                    (Joint::And, false) | (Joint::Or, true) | (Joint::Yor, _) => {
                        drop_dead_links(&mut links)
                    }
                    _ => (),
                },
                Filter::Action(_) => match (&link.joint, !link.negated) {
                    (Joint::And, false) | (Joint::Or, true) | (Joint::Yor, _) => {
                        drop_dead_links(&mut links)
                    }
                    _ => (),
                },
                _ => (),
            };

            recur |= merge_links(&mut links, link);
        });
        chain.links = links;

        if !recur {
            break;
        }
    }
    if chain.links.len() == 1 {
        reduce_singleton_chain(&mut chain);
    }
    check_actions(&mut chain);

    chain.links.shrink_to_fit();
    chain
}

fn check_actions(chain: &mut Chain) {
    chain.has_actions = false;

    for link in &chain.links {
        match link.filter {
            Filter::Action(_) => chain.has_actions = true,
            Filter::Chain(ref c) => chain.has_actions |= c.has_actions,
            _ => (),
        }
    }
}

fn drop_dead_links(links: &mut Vec<Link>) {
    while !links.is_empty() {
        if match links[links.len() - 1].filter {
            Filter::Action(_) => false,
            Filter::Chain(ref chain) => !chain.has_actions,
            _ => true,
        } {
            links.pop();
        } else {
            break;
        }
    }
}

fn merge_links(links: &mut Vec<Link>, link: Link) -> bool {
    let mut recur = false;

    match link {
        Link {
            joint,
            negated,
            filter: Filter::Chain(mut chain),
        } => {
            chain = Chain::reduce(chain);

            // this check can be changed to len() < 1
            let result = if chain.links.len() <= 1 {
                merge_singleton_link(links, joint, negated, chain)
            } else {
                let any1st = (chain.links.len() >= 2
                    && match chain.links[0].filter {
                        Filter::Anything | Filter::Action(_) => true,
                        _ => false,
                    }) as usize;
                let true1st = (any1st == 1
                    && Chain::bool(&chain.links[0].joint, true, !chain.links[0].negated))
                    as usize;
                let false1st = (any1st == 1
                    && !Chain::bool(&chain.links[0].joint, true, !chain.links[0].negated))
                    as usize;

                if false1st == 1
                    && chain.links[false1st..]
                        .iter()
                        .all(|link| link.joint == Joint::Or)
                {
                    merge_or_links(links, joint, negated, chain, false1st)
                } else if chain.links[true1st..]
                    .iter()
                    .all(|link| link.joint == Joint::And)
                {
                    merge_and_links(links, joint, negated, chain, true1st)
                } else if chain.links[any1st..]
                    .iter()
                    .all(|link| link.joint == Joint::Yor)
                {
                    merge_yor_links(links, joint, negated, chain, any1st)
                } else if chain.links[true1st | false1st..]
                    .iter()
                    .all(|link| link.joint == Joint::Xor)
                {
                    merge_xor_links(links, joint, negated, chain, true1st, false1st)
                } else if chain.links[true1st..]
                    .iter()
                    .all(|link| link.joint == Joint::Or)
                {
                    links.push(Link::new(joint, Filter::Anything, negated ^ chain.negated));
                    None
                } else {
                    Some(Link::new(joint, Filter::Chain(chain), negated))
                }
            };

            if let Some(link) = result {
                links.push(link);
            } else {
                recur = true;
            }
        }
        _ => links.push(link),
    }

    recur
}

// Chain(TRUE @ link)
fn merge_singleton_link(
    links: &mut Vec<Link>,
    joint: Joint,
    negated: bool,
    mut chain: Chain,
) -> Option<Link> {
    debug_assert!(chain.links.len() <= 1);

    let mut link = chain.links.pop().unwrap_or(Link::new(
        Joint::And, // or Joint::Yor
        Filter::Anything,
        false,
    ));

    match &link.joint {
        Joint::And | Joint::Yor => (),
        Joint::Xor => link.negated ^= true,
        Joint::Or => link.filter = Filter::Anything,
    }
    link.joint = joint;
    link.negated ^= negated ^ chain.negated;
    links.push(link);

    None
}

// Chain(TRUE & link & ...)
fn merge_and_links(
    links: &mut Vec<Link>,
    joint: Joint,
    negated: bool,
    mut chain: Chain,
    true1st: usize,
) -> Option<Link> {
    debug_assert!(chain.links.len() > 0);

    if true1st == 1 {
        chain.links[0].joint = Joint::And;
        chain.links[0].negated = false;
    }

    match (&joint, !(negated ^ chain.negated)) {
        (Joint::And, true) => {
            links.append(&mut chain.links);
            None
        }
        (Joint::Or, false) => {
            chain.links.iter_mut().for_each(|link| {
                link.joint = Joint::Or;
                link.negated ^= true;
            });
            links.append(&mut chain.links);
            None
        }
        _ => {
            let okay = match joint {
                Joint::And | Joint::Or | Joint::Xor => chain.links.len() == 1,
                Joint::Yor => true,
            };

            if okay {
                // negation affects the final result
                let idx = chain.links.len() - 1;

                chain.links[idx].negated ^= negated ^ chain.negated;
                chain.links[0].joint = joint;
                links.append(&mut chain.links);
                None
            } else if chain.negated {
                chain.links.iter_mut().for_each(|link| {
                    link.joint = Joint::Or;
                    link.negated ^= true;
                });
                chain
                    .links
                    .insert(0, Link::new(Joint::And, Filter::Anything, true));
                merge_or_links(links, joint, negated, chain.not(), 1)
            } else {
                Some(Link::new(joint, Filter::Chain(chain), negated))
            }
        }
    }
}

// Chain(FALSE | link | ...)
fn merge_or_links(
    links: &mut Vec<Link>,
    joint: Joint,
    negated: bool,
    mut chain: Chain,
    false1st: usize,
) -> Option<Link> {
    debug_assert!(chain.links.len() > 0);

    if false1st == 1 {
        chain.links[0].joint = Joint::And;
        chain.links[0].negated = true;
    }

    match (&joint, !(negated ^ chain.negated)) {
        (Joint::Or, true) => {
            chain.links.remove(0);
            links.append(&mut chain.links);
            None
        }
        (Joint::And, false) => {
            chain.links.iter_mut().for_each(|link| {
                link.joint = Joint::And;
                link.negated ^= true;
            });
            links.append(&mut chain.links);
            None
        }
        _ => {
            if chain.links.len() == 1 {
                // negation affects any short-circuited link
                chain.links[0].negated ^= negated ^ chain.negated;
                chain.links[0].joint = joint;
                links.append(&mut chain.links);
                None
            } else if chain.negated {
                chain.links.iter_mut().for_each(|link| {
                    link.joint = Joint::And;
                    link.negated ^= true;
                });
                chain.links[0].joint = Joint::And;
                chain.links[0].negated = false;
                merge_and_links(links, joint, negated, chain.not(), 1)
            } else {
                Some(Link::new(joint, Filter::Chain(chain), negated))
            }
        }
    }
}

// Chain(TRUE ^ link ^ ...) or Chain(FALSE ^ link ^ ...)
fn merge_xor_links(
    links: &mut Vec<Link>,
    joint: Joint,
    negated: bool,
    mut chain: Chain,
    true1st: usize,
    false1st: usize,
) -> Option<Link> {
    debug_assert!(chain.links.len() > 0);

    if true1st == 1 {
        chain.links[0].joint = Joint::Xor;
        chain.links[0].negated = true;
    }
    if false1st == 1 {
        chain.links[0].joint = Joint::Xor;
        chain.links[0].negated = false;
    }

    let okay = match joint {
        Joint::And | Joint::Or => chain.links.len() == 1,
        Joint::Xor | Joint::Yor => true,
    };

    if okay {
        // can be 0 because both A and B in (A XOR B) are always evaluated
        let idx = chain.links.len() - 1;

        debug_assert!(chain.links[0].joint == Joint::Xor);
        chain.links[idx].negated ^= negated ^ chain.negated ^ true;
        chain.links[0].joint = joint;
        links.append(&mut chain.links);
        None
    } else {
        Some(Link::new(joint, Filter::Chain(chain), negated))
    }
}

// Chain(TRUE $ link $ ...)
fn merge_yor_links(
    links: &mut Vec<Link>,
    joint: Joint,
    negated: bool,
    mut chain: Chain,
    any1st: usize,
) -> Option<Link> {
    debug_assert!(chain.links.len() > 0);

    let okay = match joint {
        Joint::And | Joint::Or => chain.links.len() == 1, // due to short circuit
        Joint::Xor => chain.links.len() >= 1 && chain.links.len() <= 1 + any1st,
        Joint::Yor => true,
    };

    if okay {
        // negation affects only the last predicate
        let idx = chain.links.len() - 1;

        chain.links[idx].negated ^= negated ^ chain.negated;
        chain.links[0].joint = joint;
        links.append(&mut chain.links);
        None
    } else {
        Some(Link::new(joint, Filter::Chain(chain), negated))
    }
}

// Chain(TRUE @ link)
fn reduce_singleton_chain(chain: &mut Chain) {
    debug_assert!(chain.links.len() == 1);

    let link = chain.links.pop().unwrap();

    match link {
        Link {
            joint,
            negated,
            filter: Filter::Chain(mut c),
        } => match joint {
            Joint::And | Joint::Yor => {
                c.negated ^= chain.negated ^ negated;
                *chain = c;
            }
            Joint::Xor => {
                c.negated ^= chain.negated ^ true ^ negated;
                *chain = c;
            }
            Joint::Or if !c.has_actions => (),
            _ => {
                let link = Link::new(joint, Filter::Chain(c), negated);

                chain.links.push(link);
            }
        },
        _ => chain.links.push(link),
    }
}
