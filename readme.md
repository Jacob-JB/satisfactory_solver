# Satisfactory Solver

Treats creating factories in the game Satisfactory as a linear programming problem, and is essentially just a big parser for the [minilp](https://crates.io/crates/minilp) rust crate.


### Features

Satisfactory Solver allows you to set rules and limits for what recourses and recipes a factory uses, and create an optimal design.

You can do things such as:
- Set a desired output and optimize to reduce different input resources, placing different weightings on different inputs to get the desired balance.
- Set maximum input resources and produce as many of an item as possible
- Power is an item! Measured in Mega Joules per minute. Meaning you can design power plants, and factories can include their own power production if you're doing something world scale. (make sure to divide by 60 to get Megawatts)
- Set ratios between output resources.
- Mining and sinking is treated as a recipe. You can even put limits on how many resource nodes you have.
- Pick and choose which recipes are enabled based on which alternates you have unlocked.

With all this you can design small factories or figure out what is the most efficient factory possible given the resources in the world.


### Tool Output

The output has two sections. The first is a list of all the recipes used, each recipe looking like
```md
Standard Reinforced Iron Plate: 2.75 machines
- Mega Joule: -2475/min
- Iron Plate: -82.5/min
- Screw: -165/min
- Reinforced Iron Plate: 13.75/min
```

The second section has a breakdown of what items are being used, what the net factory output/input is, and where they are being produced and consumed
```md
Reinforced Iron Plate: net 6/min
- Standard Reinforced Iron Plate: 13.75/min
- Standard Modular Frame: -3.75/min
- Standard Smart Plating: -4/min
```


### Usage

Takes command line arguments as input.

1. The first is a path to the "world" json configuration. This is where all your item and recipe configurations go, there is one included with the repo under `world.json` that includes every item and recipe short a few such as tools and non renewables. I make no garuntees of it's completeness and accuracy so feel free to open an issue if you spot an error.

2. The second argument is a path to the factory json configuration. This is where the actual setup for the factory you wish to generate goes.

3. The third argument is a path to the output, typically a `.md`. This argument is optional, if you omit it the output will be printed to the console.

e.g.
`cargo run --release -- world.json demo_configs/basic_iron.json demo_outputs/basic_iron.md`


### Examples

There are a bunch of examples under `demo_configs/` and their corresponding outputs under `demo_outputs`. Looking at these is going to be the best way to learn how to use the tool, and look into world.json if you want to add missing recipes and items or even create configurations for modded playthroughs.


### Tips

here are a few notes and tips that you might want to know when using the tool

- By default, a rule for a net production of zero is added to every item that doesn't have another rule. Make sure that *everying* you expect your factory to consume and produce has a rule. Add a no default rule `{"NoDefault": "Mega Joule"}` if you want no limits.
- Negative numbers mean consumption, positive numbers are production
- Rules can be set for recipe usage not just items. E.g. in the case you wanted to run 10 fuel generators for example. Just note that this is in "conversions per minute" not "number of machines", check the world config for that exact recipes rates.
- In a few cases such as for the recipe `Standard Encased Uranium Cell` that produce and consume the same item, only the net effect is shown.
- All the sink recipes assume Mk5 belts. Add other recipes if this is an issue.
- You add recipe names to the enabled recipes list, or their tags.
- If you want to link the ratio of production between two items, say 4 assembly director systems per 1 nuclear pasta, you can add an equation rule shown below

`(Assembly Director System)/(Magnetic Field generator) = 4/1`

Expressed as a linear equation:

`1(Assembly Director System) - 4(Magnetic Field generator) = 0`

In json:

```json
{"Equation": [
    [
        ["Assembly Director System", 1],
        ["Magnetic Field Generator", -4]
    ],
    "=",
    0
]}
```
