#!/bin/python

import json
import requests

host = "localhost"
port = "8080"

payload = {"game":{"id":"3da3b81c-7524-4b0b-83c9-5a076ad31639","ruleset":{"name":"standard","version":"?","settings":{"foodSpawnChance":15,"minimumFood":1}},"map":"standard","timeout":500,"source":"custom"},"turn":326,"board":{"width":11,"height":11,"food":[{"x":5,"y":10},{"x":4,"y":7},{"x":3,"y":2},{"x":10,"y":10},{"x":10,"y":5}],"hazards":[],
    "snakes":[
        {"next_move": "left","id":"gs_ptJ884x3j9gqxvXtQ8wqwYJT","name":"Shapeshifter","health":88,"body":[{"x":1,"y":1},{"x":1,"y":0},{"x":2,"y":0},{"x":3,"y":0},{"x":4,"y":0},{"x":5,"y":0},{"x":6,"y":0},{"x":6,"y":1},{"x":6,"y":2},{"x":7,"y":2},{"x":7,"y":1},{"x":7,"y":0},{"x":8,"y":0},{"x":8,"y":1},{"x":8,"y":2},{"x":8,"y":3},{"x":9,"y":3},{"x":9,"y":4},{"x":9,"y":5},{"x":9,"y":6},{"x":9,"y":7},{"x":9,"y":8},{"x":9,"y":9},{"x":9,"y":10},{"x":8,"y":10},{"x":8,"y":9},{"x":7,"y":9},{"x":7,"y":8},{"x":8,"y":8}],"latency":413,"head":{"x":1,"y":1},"length":29,"shout":"","squad":"","customizations":{"color":"#900050","head":"cosmic-horror-special","tail":"cosmic-horror"}},
        {"next_move": "left","id":"gs_SKFvVjGqwSr3dBYbwF8VHBbJ","name":"Hovering Hobbs","health":100,"body":[{"x":6,"y":10},{"x":6,"y":9},{"x":6,"y":8},{"x":6,"y":7},{"x":7,"y":7},{"x":7,"y":6},{"x":6,"y":6},{"x":5,"y":6},{"x":5,"y":5},{"x":4,"y":5},{"x":3,"y":5},{"x":2,"y":5},{"x":2,"y":4},{"x":1,"y":4},{"x":1,"y":3},{"x":2,"y":3},{"x":2,"y":2},{"x":1,"y":2},{"x":0,"y":2},{"x":0,"y":2}],"latency":394,"head":{"x":6,"y":10},"length":20,"shout":"","squad":"","customizations":{"color":"#da8a1a","head":"beach-puffin-special","tail":"beach-puffin-special"}}
    ]},"you":{"id":"gs_ptJ884x3j9gqxvXtQ8wqwYJT","name":"Shapeshifter","health":88,"body":[{"x":1,"y":1},{"x":1,"y":0},{"x":2,"y":0},{"x":3,"y":0},{"x":4,"y":0},{"x":5,"y":0},{"x":6,"y":0},{"x":6,"y":1},{"x":6,"y":2},{"x":7,"y":2},{"x":7,"y":1},{"x":7,"y":0},{"x":8,"y":0},{"x":8,"y":1},{"x":8,"y":2},{"x":8,"y":3},{"x":9,"y":3},{"x":9,"y":4},{"x":9,"y":5},{"x":9,"y":6},{"x":9,"y":7},{"x":9,"y":8},{"x":9,"y":9},{"x":9,"y":10},{"x":8,"y":10},{"x":8,"y":9},{"x":7,"y":9},{"x":7,"y":8},{"x":8,"y":8}],"latency":413,"head":{"x":1,"y":1},"length":29,"shout":"","squad":"","customizations":{"color":"#900050","head":"cosmic-horror-special","tail":"cosmic-horror"}}}
resp = requests.post(f"http://{host}:{port}/debug/simulate_turn", json=payload)

print(resp)
print(resp.json())
