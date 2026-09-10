# Tank models

Put self-contained **`.glb`** files here. The game loads them at startup:

```
assets/tanks/models/tank_1_green.glb
assets/tanks/models/tank_1_bw.glb
assets/tanks/models/tree.glb
assets/tanks/models/tree2.glb
assets/tanks/models/tree3.glb
assets/tanks/models/tree4.glb
```

If a file is missing, that mesh falls back to a box.

## Credits

- Tanks: [FREE stylized tank 3D model](https://mreliptik.itch.io/free-lowpoly-tank-3d-model) by [MrEliptik](https://mreliptik.itch.io)
- Trees: [10+ Free Low Poly Trees Pack](https://crazydrpants.itch.io/free-low-poly-trees-pack) by [CrazyDrPants](https://crazydrpants.itch.io)

## FBX → GLB

Runtime reads **GLB/glTF only**. Convert in Blender:

1. File → Import → FBX
2. File → Export → glTF 2.0
3. Format: **glTF Binary (.glb)**
4. Transform: **+Y Up**
5. Embed textures (PNG/JPEG inside the GLB). External `.bin` / URI files are not fetched.

You can also drop a `.glb` onto the game window (native) or the canvas (web).
