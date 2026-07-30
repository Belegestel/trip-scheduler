# Trip planner

> Given a list of places to visit and a basic trip outline, this program finds the pareto optimal itineraries, balancing the number of optional places visited and the time spent outside.

This project is entirely practical - I created it for my upcoming trip. I wanted to ensure that I will be able to see as many places as possible within a fixed time budget. This means that for every day I spend at my destination, I want to find the optimal subset of destinations to visit. However, this is not a simple optimization task: I am also optimizing the number of optional destinations - I have divided the set of destinations into mandatory and optional locations. Therefore, the program optimizes both the time spent walking (minimization) and the number of optional destinations seen (maximization).

## Assumptions 
- Every day starts and ends at the same origin
- Travel times are symmetric
- Every destination is visited at most once
- Visit durations are fixed

## Nerd talk
This is a multi-criteria optimization task. Therefore, my goal is to find a *pareto frontier*, where on one axis I can see the lowest maximum time spent travelling in a day, and the other axis describes the number of optional destinations. This lets me decide which subset works best for my own requirements.

It is worth specifying the objectives more cleanly. The criteria are:
1. Maximum time spent outside (minimization)
2. Number of optional locations visited (maximization)

The problem is solved in a number of stages, as follows:

### Travelling Salesman problem
To evaluate all possible subsets of destinations, I am using https://openrouteservice.org/ to get the distance matrix between all destinations. Then I run the Held-Karp algorithm to compute the distance for all possible subsets. At the end, I also add to every result the static value assigned to every destination - time spent upon arrival.
This step is very quick and runs in **less than 10s**, despite the $O(n^2 \cdot 2^n)$ complexity.

### Branch and bound 
The second stage focuses on using branch and bound to find the best itinerary. As the evaluation of a single solution is incredibly fast (due to having already precomputed all TSP subsets), the first step is actually focusing on randomly finding a decent upper bound - the number of computations is defined by a config variable `upper_bound_random_size` (I kept it at `1 000 000`). Afterwards, a multithreaded processing begins - one thread for each possible number of additional destinations.
This step is not the fastest - in my case, with 20 cities (and roughly half of them being optional) being distributed across five days, the program ran for about 10 minutes. This step is exponential in the worst case, but pruning significantly helps it accelerate to acceptable execution times.

### Output
The solver produces a `.json` file, that contains the following information:
- location names
- for every number of optional locations:
    - daily travel time for an optimal solution 
    - optimal destination assignment for every day

# Usage
In advance, apologies for the messy setup required - I didn't plan on uploading this anywhere at first, therefore the program runs well in my environment. 

## Folder setup

```
├── places.csv
├── config.json
├── .env
├── data/
├── results/
│   └── parsed/
├── src/
├── plotter.py
└── Cargo.toml
```
In the `data` folder, there will be a single file - `cache.json`. It stores data from the API. If you change your `places.csv`, please remove the `.json` file, as it will feed the program cached data that doesn't correspond to the input data and it might cause unpredictable behavior.
In the `results` folder, all solver's solutions will be placed (timestamped).
In the `results/parsed` folder, all the parser's data will be placed. This data is human readable and is the final product of this project.

## Input file: `places.csv`

This file contains all destinations you wish to visit. Place it in the root folder.
The structure is as follows:
```
Location name;Lat;Lon;Obligatory;Duration
```
- `Location name`: label, by which you'll be able to identify the destination
- `Lat`: Latitude
- `Lon`: Longitude
- `Obligatory`: `0` for optional destinations, `1` for mandatory
- `Duration`: time spent (in minutes) at destination - if it's a museum, it's probably worth spending a while in there!

Example:
```
Location name;Lat;Lon;Obligatory;Duration
A pretty castle;12.345;54.321;1;60
An interesting sight;12.543;54.123;0;5
```

## Config file: `config.json`

This file defines the program behavior and should be placed in the program root folder.

Template:
```
{
  "origin": [ LAT, LON ],
  "day_weights": [ THIS, IS, MY, DAY, WEIGHT ],
  "day_limit": 720.0,
  "upper_bound_random_size": 1000000
}
```

- `origin`: This is the location the program assumes you come back to every day 
- `day_weights`: A sequence of integers. Every single one describes what part of the day is available for you, and the program will try to distribute the tasks in such a way, that the distribution corresponds to the weights. Acceptable values are between `0.0` (exclusive) and `1.0` (inclusive)
- `day_limit`: absolute maximum time in a day you want to spend outside (in minutes). If unlimited, set to `1440` - 24h
- `upper_bound_random_size`: the number of randomly sampled locations before running branch and bound algorithm. This step is very fast, so 1 million is easily achievable.

## Environmental variables: the `.env` file 

This file defines API keys and should be placed in the program root folder.
The API key can be obtained on the https://openrouteservice.org/ website
```
API_KEY=
```

Example
```
API_KEY="THIS-IS-MY-SECRET-API-KEY-123"
```

## Running the code 

```sh
# This automatically installs the only python dependency
python3 -m pip install matplotlib 

# This automatically installs all Rust dependencies
cargo run

# This generates the human-readable summary from the solver results
# Pass as the argument the path to the sovler results in ./results/
python3 ./plotter.py ./results/123456789.json 
```
