"""Blender import of mb3d-rs exports: a mesh (.ply or .obj from the Bulb
Tracer window or `mb3d mesh`) with its vertex colours as the material, or
a Gaussian splat / point cloud .ply as points.

In Blender: Scripting workspace, open this file, set FILES below (or run
with the files as arguments after `--`), Run Script.  Or from a terminal:

    blender --python tools/blender_import_mb3d.py -- bulb.ply other.obj

What it does for a mesh:
  - imports the file (PLY or OBJ; Blender 4.x and 3.x operators),
  - recalculates the normals, shades smooth,
  - gives it a material whose base colour is the vertex colour attribute
    (mb3d-rs writes the colours per vertex when "Calculate colors" is on,
    or --colors on the command line), so it renders coloured in Eevee and
    Cycles without any texture,
  - centres it on the origin.

A splat .ply (the Bulb Tracer's splat export) has no faces: it comes in as
a point cloud; a Geometry Nodes "Mesh to Points" with a small radius makes
it visible in Cycles.

Blender 3.0 or newer.
"""
import sys
import os

FILES = []  # e.g. ["/home/me/bulb.ply"]


def _args():
    if "--" in sys.argv:
        return sys.argv[sys.argv.index("--") + 1:]
    return FILES


def import_file(path):
    import bpy
    ext = os.path.splitext(path)[1].lower()
    before = set(bpy.data.objects)
    if ext == ".ply":
        if hasattr(bpy.ops.wm, "ply_import"):
            bpy.ops.wm.ply_import(filepath=path)
        else:
            bpy.ops.import_mesh.ply(filepath=path)
    elif ext == ".obj":
        if hasattr(bpy.ops.wm, "obj_import"):
            bpy.ops.wm.obj_import(filepath=path)
        else:
            bpy.ops.import_scene.obj(filepath=path)
    elif ext == ".stl":
        if hasattr(bpy.ops.wm, "stl_import"):
            bpy.ops.wm.stl_import(filepath=path)
        else:
            bpy.ops.import_mesh.stl(filepath=path)
    else:
        raise ValueError(f"{path}: not a .ply, .obj or .stl file")
    new = [o for o in bpy.data.objects if o not in before and o.type == "MESH"]
    return new


def color_attribute(mesh):
    """The vertex colour attribute of the mesh, whatever its name."""
    attrs = getattr(mesh, "color_attributes", None)
    if attrs and len(attrs):
        return attrs[0].name
    if mesh.vertex_colors:
        return mesh.vertex_colors[0].name
    return None


def vertex_color_material(name, attr):
    import bpy
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    links = mat.node_tree.links
    bsdf = nodes.get("Principled BSDF")
    col = nodes.new("ShaderNodeVertexColor")
    col.layer_name = attr
    col.location = (-300, 300)
    links.new(col.outputs["Color"], bsdf.inputs["Base Color"])
    bsdf.inputs["Roughness"].default_value = 0.6
    return mat


def prepare(obj):
    import bpy
    mesh = obj.data
    if len(mesh.polygons) == 0:
        # a splat or point export: points, shown by a Mesh to Points node
        print(f"{obj.name}: {len(mesh.vertices)} points, no faces (a splat or point cloud)")
        mod = obj.modifiers.new("Points", "NODES")
        try:
            bpy.ops.node.new_geometry_node_group_assign()
        except Exception:
            pass
        return
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.mesh.normals_make_consistent(inside=False)
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.shade_smooth()
    attr = color_attribute(mesh)
    if attr:
        mat = vertex_color_material(f"{obj.name} colours", attr)
        if mesh.materials:
            mesh.materials[0] = mat
        else:
            mesh.materials.append(mat)
        print(f"{obj.name}: {len(mesh.vertices)} vertices, {len(mesh.polygons)} faces, vertex colours '{attr}' as the material")
    else:
        print(f"{obj.name}: {len(mesh.vertices)} vertices, {len(mesh.polygons)} faces, no vertex colours")
    bpy.ops.object.origin_set(type="ORIGIN_GEOMETRY", center="BOUNDS")
    obj.location = (0.0, 0.0, 0.0)


def main():
    files = _args()
    if not files:
        print("no files: set FILES in the script or pass them after --")
        return
    for f in files:
        for obj in import_file(os.path.abspath(f)):
            prepare(obj)


if __name__ == "__main__":
    main()
