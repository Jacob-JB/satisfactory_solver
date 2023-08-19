
use std::{collections::{HashMap, HashSet}, time::Instant, fs::File, io::Write};

use minilp::{Problem, ComparisonOp};
use serde::Deserialize;

#[derive(Deserialize)]
struct RawWorldConfig {
    items: Vec<String>,
    recipes: Vec<RecipeGroup>,
}

#[derive(Deserialize)]
struct RecipeGroup {
    tags: Vec<String>,
    recipes: Vec<Recipe>
}

#[derive(Default)]
struct WorldConfig {
    items: Vec<String>,
    recipes: Vec<Recipe>,
}

#[derive(Deserialize)]
struct Recipe {
    name: String,
    per_machine: f64,
    tags: Vec<String>,
    items: Vec<(String, f64)>,
}

#[derive(Deserialize)]
struct SolverConfig {
    round_zeros: i32,
    rules: Vec<Rule>,
    optimize: Vec<(String, f64)>,
    enabled_recipes: Vec<String>,
}

#[derive(Deserialize)]
enum Rule {
    Equal(String, f64),
    LessThan(String, f64),
    GreaterThan(String, f64),
    NoDefault(String),
    Equation(Vec<(String, f64)>, String, f64),
}

fn main() {
    let mut args = std::env::args();
    args.next();

    let Some(world_file_location) = args.next() else {
        println!("expected file path to world config as first argument");
        return;
    };

    let Some(solver_file_location) = args.next() else {
        println!("expected file path to solver config as second argument");
        return;
    };


    let Ok(file) = std::fs::read_to_string(world_file_location.as_str()) else {
        println!("could not find world config at \"{}\"", world_file_location);
        return;
    };

    let Ok(world_config) = serde_json::from_str::<RawWorldConfig>(file.as_str()) else {
        println!("invalid world config format");
        return;
    };


    let Ok(file) = std::fs::read_to_string(solver_file_location.as_str()) else {
        println!("could not find solver config at \"{}\"", solver_file_location);
        return;
    };

    let Ok(solver_config) = serde_json::from_str::<SolverConfig>(file.as_str()) else {
        println!("invalid solver config format");
        return;
    };


    let recipe_groups = world_config.recipes;
    let mut world_config = WorldConfig {
        items: world_config.items,
        recipes: Vec::new(),
    };

    for group in recipe_groups {
        for mut recipe in group.recipes {
            recipe.tags.append(&mut group.tags.clone());
            world_config.recipes.push(recipe);
        }
    }


    // filter for enabled recipes
    if !solver_config.enabled_recipes.contains(&"All".into()) {
        world_config.recipes = world_config.recipes.into_iter().filter(
            |recipe| {
                if solver_config.enabled_recipes.contains(&recipe.name) {return true;}
                for tag in recipe.tags.iter() {
                    if solver_config.enabled_recipes.contains(&tag) {return true;}
                }
                false
            }
        ).collect();
    }


    // create name -> id map
    // resources then recipes
    let mut id_map = HashMap::new();
    let mut variable_count = 0;


    for item in world_config.items.iter() {
        if id_map.insert(item, variable_count).is_some() {
            println!("duplicate naming \"{}\"", item);
            return;
        }
        variable_count += 1;
    }

    for recipe in world_config.recipes.iter() {
        if id_map.insert(&recipe.name, variable_count).is_some() {
            println!("duplicate naming \"{}\"", recipe.name);
            return;
        }
        variable_count += 1;
    }


    let item_count = world_config.items.len();


    // create problem and objective function
    let mut objective_coefficients = vec![0.; variable_count];

    for (name, coefficient) in solver_config.optimize.iter() {
        if let Some(&id) = id_map.get(name) {
            objective_coefficients[id] = *coefficient;
        } else {
            println!("invalid name \"{}\" in optimize list", name);
            return;
        }
    }


    let mut problem = Problem::new(minilp::OptimizationDirection::Maximize);
    let variables: Vec<_> = objective_coefficients.into_iter().map(|c| problem.add_var(c, (f64::NEG_INFINITY, f64::INFINITY))).collect();


    // limit recipe usage to be positive
    for recipe in world_config.recipes.iter() {
        let id = id_map[&recipe.name];
        problem.add_constraint([(variables[id], 1.)], ComparisonOp::Ge, 0.)
    }


    // limit net item usage to be the sum of all recipes
    let mut item_sum_coefficients = vec![Vec::new(); item_count];

    for recipe in world_config.recipes.iter() {
        let recipe_var = variables[id_map[&recipe.name]];
        for (name, coefficient) in recipe.items.iter() {
            if let Some(&id) = id_map.get(name) {
                item_sum_coefficients[id].push((recipe_var, *coefficient));
            } else {
                println!("invalid name \"{}\" in recipe \"{}\"", name, recipe.name);
                return;
            }
        }
    }


    // don't allow double ups in recipe items
    for recipe in world_config.recipes.iter() {
        let mut found: Vec<&String> = Vec::new();
        for (name, _) in recipe.items.iter() {
            if found.iter().any(|e| **e == *name) {
                println!("duplicate item \"{}\" in recipe \"{}\"", name, recipe.name);
                return;
            }
            found.push(name);
        }
    }


    // add coefficients from previous step
    for (item_id, mut coefficients) in item_sum_coefficients.into_iter().enumerate() {
        coefficients.push((variables[item_id], -1.));
        problem.add_constraint(coefficients, ComparisonOp::Eq, 0.);
    }


    // add rule constraints
    let mut defaults: HashSet<_> = world_config.items.iter().collect();

    for (i, rule) in solver_config.rules.iter().enumerate() {
        match rule {
            Rule::Equal(name, rhs) => {
                if let Some(&id) = id_map.get(name) {
                    problem.add_constraint([(variables[id], 1.)], ComparisonOp::Eq, *rhs);
                    defaults.remove(name);
                } else {
                    println!("invalid name \"{}\" in rule {}", name, i);
                    return;
                }
            },

            Rule::LessThan(name, rhs) => {
                if let Some(&id) = id_map.get(name) {
                    problem.add_constraint([(variables[id], 1.)], ComparisonOp::Le, *rhs);
                    defaults.remove(name);
                } else {
                    println!("invalid name \"{}\" in rule {}", name, i);
                    return;
                }
            },

            Rule::GreaterThan(name, rhs) => {
                if let Some(&id) = id_map.get(name) {
                    problem.add_constraint([(variables[id], 1.)], ComparisonOp::Ge, *rhs);
                    defaults.remove(name);
                } else {
                    println!("invalid name \"{}\" in rule {}", name, i);
                    return;
                }
            }

            Rule::NoDefault(name) => {
                if id_map.contains_key(name) {
                    defaults.remove(name);
                } else {
                    println!("invalid name \"{}\" in rule {}", name, i);
                    return;
                }
            }

            Rule::Equation(named_coefficients, comparison, rhs) => {
                let mut coefficients = Vec::new();
                for (name, coefficient) in named_coefficients {
                    if let Some(&id) = id_map.get(name) {
                        coefficients.push((variables[id], *coefficient));
                    } else {
                        println!("invalid name \"{}\" in rule {}", name, i);
                        return;
                    }
                }
                problem.add_constraint(coefficients, match comparison.as_str() {
                    "=" => ComparisonOp::Eq,
                    ">" => ComparisonOp::Ge,
                    "<" => ComparisonOp::Le,
                    comparison => {
                        println!("invalid equation operator \"{}\" in rule {}", comparison, i);
                        return;
                    }
                }, *rhs);
            }
        }
    }


    // default any items without a comparison rule to equal zero
    for name in defaults {
        problem.add_constraint([(variables[id_map[name]], 1.)], ComparisonOp::Eq, 0.)
    }


    // solve
    println!("config is valid, solving");
    let start = Instant::now();
    let solution = problem.solve();
    println!("done in {:#?}", start.elapsed());

    let Ok(solution) = solution else {
        println!("impossible setup");
        return;
    };

    println!("successful");

    let mut output_file = if let Some(output_file) = args.next() {
        if let Ok(output_file) = File::create(output_file) {
            Some(output_file)
        } else {
            println!("could not open output file, printing output to console");
            None
        }
    } else {
        println!("printing output to console, you can specify a third argument to write the output to");
        None
    };

    if output_file.is_none() {
        println!("output:\n");
    }

    let mut item_usages = vec![Vec::new(); item_count];

    outputln(output_file.as_mut(), "### recipe usage");
    for (i, recipe) in world_config.recipes.iter().enumerate() {
        let rate = solution[variables[item_count + i]];

        if rate < 0.1f64.powi(solver_config.round_zeros) {continue;}

        outputln(output_file.as_mut(), format!("\n{}: {} machines", recipe.name, round(rate / recipe.per_machine, solver_config.round_zeros)).as_str());
        for (name, coefficient) in recipe.items.iter() {
            let item_rate = rate * coefficient;
            outputln(output_file.as_mut(), format!("- {}: {}/min", name, round(item_rate, solver_config.round_zeros)).as_str());
            item_usages[id_map[name]].push((&recipe.name, item_rate));
        }
    }

    outputln(output_file.as_mut(), format!("\n\n### item throughput").as_str());
    for (id, (name, usages)) in world_config.items.iter().zip(item_usages.into_iter()).enumerate() {
        if usages.len() == 0 {continue;}

        outputln(output_file.as_mut(), format!("\n{}: net {}/min", name, round(solution[variables[id]], solver_config.round_zeros)).as_str());
        for (name, rate) in usages {
            outputln(output_file.as_mut(), format!("- {}: {}/min", name, round(rate, solver_config.round_zeros)).as_str());
        }
    }
}


fn round(value: f64, zeros: i32) -> f64 {
    let exponent = 10f64.powi(zeros);
    (value * exponent + 0.5).floor() / exponent
}


fn outputln(file: Option<&mut File>, output: &str) {
    if let Some(file) = file {
        for byte in output.bytes() {
            file.write(&[byte]).expect("failed to write to file");
        }
        file.write(b"\n").expect("failed to write to file");
    } else {
        println!("{}", output);
    }
}
