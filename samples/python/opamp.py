 #!/usr/bin/env python

import io
from IPython.display import SVG, display
from PIL import Image
import recad

schema = recad.Schema("")
schema.move_to((50.8, 50.8))
schema = (schema
    + recad.LocalLabel("Vin").rotate(180) 
    + recad.Wire().right()
    + recad.Symbol("R1", "100k", "Device:R").rotate(90)
    + recad.Junction()
    + recad.Symbol("U1", "TL072", "Amplifier_Operational:LM2904").anchor("2").mirror("x")
    + recad.Junction()
    + recad.Wire().up().length(4)
    + recad.Symbol("R2", "100k", "Device:R").rotate(270).tox("U1", "2")
    + recad.Wire().toy("U1", "2")
    + recad.Symbol("GND", "GND", "power:GND").at("U1", "3")
    + recad.LocalLabel("Vout").at("U1", "1")
)

svg = schema.plot(scale=10)
path = "/tmp/recad/py_opamp.svg"
schema.plot(scale=10, path = path)
print(f"Image writter to {path}")
