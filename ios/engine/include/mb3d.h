/* libmb3d: 3D fractals of Mandelbulb3D, calculated by mb3d-rs.
 * EXPERIMENTAL: the functions may still change.
 *
 *     mb3d_scene *scene = mb3d_preset("Amazing Box");
 *     mb3d_scene_set(scene, "zoom = 0.3\niterations = 12");
 *     mb3d_scene_resize(scene, 1280, 960);
 *     mb3d_picture *picture = mb3d_render(scene, NULL, NULL);
 *     mb3d_picture_save_png(picture, "box.png");
 *     mb3d_picture_free(picture);
 *     mb3d_scene_free(scene);
 *
 * Texts are UTF-8 and end with a zero byte.  A failed call returns NULL (or
 * -1) and mb3d_last_error() tells why, on the same thread.  A scene is used
 * by one thread at a time; different scenes may be used by different
 * threads.
 */
#ifndef MB3D_H
#define MB3D_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct mb3d_scene mb3d_scene;
typedef struct mb3d_picture mb3d_picture;

/* This library's version ("0.1.0") and the engine's (the mb3d-rs release it
 * was built from).  Not to be freed. */
const char *mb3d_version(void);
const char *mb3d_engine_version(void);

/* Why the last call on this thread failed.  Valid until the next failed call
 * on this thread; not to be freed. */
const char *mb3d_last_error(void);

/* Calculate on the graphics card when the scene allows it (1), or on the
 * processor only (0, how the library starts). */
void mb3d_use_gpu(int on);

/* To call before the program ends when the graphics card was used: waits
 * until no formula is being compiled for the card any more (a process that
 * ends inside the driver crashes there). */
void mb3d_finish(void);

/* A further folder with formula files (.m3f). */
int mb3d_add_formula_folder(const char *dir);
/* A further folder with map pictures. */
int mb3d_add_map_folder(const char *dir);

/* A scene from a parameter file (.m3p, .m3i, MB3D text parameters, a scene
 * text), from a scene text, from a file's bytes (name tells the kind, like a
 * file name), or a formula by name with a camera that shows it.  Free it
 * with mb3d_scene_free. */
mb3d_scene *mb3d_open(const char *path);
mb3d_scene *mb3d_open_text(const char *scene_text);
mb3d_scene *mb3d_open_bytes(const char *name, const uint8_t *data, size_t len);
mb3d_scene *mb3d_preset(const char *formula);
mb3d_scene *mb3d_scene_copy(const mb3d_scene *scene);
void mb3d_scene_free(mb3d_scene *scene);

/* Changes settings: `key = value` lines of global keys, as mb3d_scene_text
 * shows them.  A wrong line changes nothing (-1). */
int mb3d_scene_set(mb3d_scene *scene, const char *changes);

/* Changes one of a formula's own values by name ("Scale", "Integer power",
 * ...: the [formula] sections of mb3d_scene_text show them); slot is the
 * formula's place in the scene, 0 for the first. */
int mb3d_scene_set_formula(mb3d_scene *scene, uint32_t slot, const char *key, double value);

/* The scene as `key = value` lines; free it with mb3d_string_free. */
char *mb3d_scene_text(const mb3d_scene *scene);
void mb3d_string_free(char *text);

/* The picture size in pixels; another size keeps the look of the picture. */
int mb3d_scene_size(const mb3d_scene *scene, uint32_t *width, uint32_t *height);
int mb3d_scene_resize(mb3d_scene *scene, uint32_t width, uint32_t height);

/* The view as the three angles of the `rotation` key, in degrees; -1 for a
 * view they cannot give back. */
int mb3d_scene_rotation(const mb3d_scene *scene, double angles[3]);

/* Told the stage ("" during the march, then "hard shadows", ...) and its
 * work done and to do; returns non-zero to stop the calculation.  Called
 * from the engine's threads, one call at a time. */
typedef int (*mb3d_progress)(void *user, const char *stage, size_t done, size_t total);

/* Calculates the picture (progress may be NULL).  NULL when it failed or
 * was stopped (mb3d_last_error: "cancelled").  Free it with
 * mb3d_picture_free. */
mb3d_picture *mb3d_render(const mb3d_scene *scene, mb3d_progress progress, void *user);

uint32_t mb3d_picture_width(const mb3d_picture *picture);
uint32_t mb3d_picture_height(const mb3d_picture *picture);
/* width * height * 3 bytes: red, green, blue, rows from the top.  Valid
 * until the picture is freed. */
const uint8_t *mb3d_picture_rgb(const mb3d_picture *picture);
/* width * height numbers: 65535 nearest, 0 the background. */
const uint16_t *mb3d_picture_depth(const mb3d_picture *picture);
/* How long the calculation took. */
double mb3d_picture_seconds(const mb3d_picture *picture);
int mb3d_picture_save_png(const mb3d_picture *picture, const char *path);
void mb3d_picture_free(mb3d_picture *picture);

/* The requests of `mb3d serve` (mb3d-rs: docs/API.md) without the program,
 * for wrappers in other languages: a session keeps the scenes its requests
 * open.  The Python module is built on these four functions. */
typedef struct mb3d_session mb3d_session;

/* Told every line that comes before a request's answer (progress), as JSON.
 * Called from the engine's threads, one call at a time. */
typedef void (*mb3d_event)(void *user, const char *json);

mb3d_session *mb3d_session_new(void);
/* One request as JSON ({"op":"open","preset":"Amazing Box"}); the answer as
 * JSON, to free with mb3d_string_free.  A request that failed answers
 * {"ok":false,"error":...}; NULL only for a request that is not JSON.  One
 * request at a time: a second thread waits.  event may be NULL. */
char *mb3d_session_request(const mb3d_session *session, const char *request, mb3d_event event, void *user);
/* Stops the session's render that is running (from another thread): it
 * answers "cancelled". */
void mb3d_session_cancel(const mb3d_session *session);
void mb3d_session_free(mb3d_session *session);

/* The navigator: a camera flying through a scene, moved by steps measured
 * with the distance to the surface (MB3D's Navigator without its window).
 * One thread at a time. */
typedef struct mb3d_navigator mb3d_navigator;

/* A camera starting at the scene's view; free with mb3d_navigator_free. */
mb3d_navigator *mb3d_navigator_new(const mb3d_scene *scene);
void mb3d_navigator_free(mb3d_navigator *nav);
/* Turns the camera where it stands, in degrees: a drag of (dx, dy) pixels
 * turns by pitch = -dy * k, yaw = dx * k, k = field of view / height. */
void mb3d_navigator_turn(mb3d_navigator *nav, double pitch, double yaw, double roll);
/* Slides sideways by a drag of (dx, dy) picture heights. */
void mb3d_navigator_slide(mb3d_navigator *nav, double dx, double dy);
/* Flies forward (backward when negative) by that percentage of the
 * distance to the surface (at most 90). */
void mb3d_navigator_fly(mb3d_navigator *nav, double percent);
/* The field of view in degrees. */
double mb3d_navigator_fov(const mb3d_navigator *nav);
/* A new scene of the view at width x height: quick != 0 the quick one for
 * while the camera moves (simpler light), else the full one; free it with
 * mb3d_scene_free. */
mb3d_scene *mb3d_navigator_scene(const mb3d_navigator *nav, int quick, uint32_t width, uint32_t height);

/* --- formulas and files (iPad app) --- */

/* The names of all formulas, one per line; free with mb3d_string_free. */
char *mb3d_formula_names(void);
/* A formula's settings with their defaults as `key = value` lines of a
 * [formula] section; free with mb3d_string_free. */
char *mb3d_formula_settings(const char *name);
/* Writes the scene as a Mandelbulb3D parameter file (.m3p); 0 or -1. */
int mb3d_scene_save_m3p(const mb3d_scene *scene, const char *path);
/* The parameters as Mandelbulb3D text (Mandelbulb3Dv18{...}) for the
 * clipboard; free with mb3d_string_free. */
char *mb3d_scene_mb3d_text(const mb3d_scene *scene, const char *title);
/* The depth at pixel (x, y) as dof_focus takes it, into *focus: 0, or -1
 * on the background. */
int mb3d_scene_pick(const mb3d_scene *scene, uint32_t x, uint32_t y, double *focus);
/* MutaGen: 15 variations of the scene into out[15] (each freed with
 * mb3d_scene_free; out[0] is the scene itself; the tree: 1, 1.1, 1.2, then
 * their children); progress as mb3d_render's. 0, or -1. */
int mb3d_variations(const mb3d_scene *scene, uint64_t seed, mb3d_scene **out, mb3d_progress progress, void *user);

/* Monte Carlo: soft light by random rays, pass after pass less noisy. */
typedef struct mb3d_montecarlo mb3d_montecarlo;
mb3d_montecarlo *mb3d_montecarlo_new(const mb3d_scene *scene);
void mb3d_montecarlo_free(mb3d_montecarlo *mc);
/* One more pass; progress as mb3d_render's. 0, or -1 (stopped, failed). */
int mb3d_montecarlo_pass(mb3d_montecarlo *mc, mb3d_progress progress, void *user);
uint32_t mb3d_montecarlo_passes(const mb3d_montecarlo *mc);
/* The picture so far; free with mb3d_picture_free. */
mb3d_picture *mb3d_montecarlo_picture(const mb3d_montecarlo *mc);

/* A JPEG of a w x h equirectangular picture with the tags of a 360 photo
 * (Photo Sphere XMP, 360 camera EXIF); its length into *out_len; free with
 * mb3d_bytes_free. */
uint8_t *mb3d_jpeg_360(const uint8_t *jpeg, size_t len, uint32_t w, uint32_t h, size_t *out_len);
void mb3d_bytes_free(uint8_t *p, size_t len);

/* Animation: keyframes and the frames between them. */
typedef struct mb3d_film mb3d_film;
/* looped != 0 back to the first; interpolation 0 linear, 1 Bezier,
 * 2 Catmull-Rom. */
mb3d_film *mb3d_film_new(int looped, int interpolation);
/* A turntable around the object: steps keyframes, frames each. */
mb3d_film *mb3d_film_orbit(const mb3d_scene *scene, uint32_t steps, uint32_t frames);
void mb3d_film_free(mb3d_film *film);
int mb3d_film_add(mb3d_film *film, const mb3d_scene *scene, uint32_t frames);
uint32_t mb3d_film_keyframes(const mb3d_film *film);
mb3d_scene *mb3d_film_keyframe(const mb3d_film *film, uint32_t i);
uint32_t mb3d_film_frames(const mb3d_film *film);
/* The scene of frame i (from 0); free with mb3d_scene_free. */
mb3d_scene *mb3d_film_frame(const mb3d_film *film, uint32_t i);
/* The scene offset (0..1) of the way to the next frame (motion blur). */
mb3d_scene *mb3d_film_frame_at(const mb3d_film *film, uint32_t i, double offset);
/* Curves: the values over the frames. Names one per line (free with
 * mb3d_string_free); a curve's values at every frame (values: room for
 * mb3d_film_frames, NaN where none) and at the keyframes (room for
 * mb3d_film_keyframes each). */
char *mb3d_curve_names(void);
int mb3d_film_curve(const mb3d_film *film, uint32_t param, double *values, uint32_t *key_frames, double *key_values);
int mb3d_film_set_curve_key(mb3d_film *film, uint32_t param, uint32_t key, double value);

/* 3D model: the BulbTracer's triangle mesh. */
typedef struct mb3d_mesh mb3d_mesh;
mb3d_mesh *mb3d_mesh_trace(const mb3d_scene *scene, uint32_t resolution, int colors, uint32_t smooth, double size,
                           mb3d_progress progress, void *user);
void mb3d_mesh_free(mb3d_mesh *mesh);
size_t mb3d_mesh_vertex_count(const mb3d_mesh *mesh);
size_t mb3d_mesh_face_count(const mb3d_mesh *mesh);
/* 3 floats per vertex; valid while the mesh lives. */
const float *mb3d_mesh_vertices(const mb3d_mesh *mesh);
const float *mb3d_mesh_normals(const mb3d_mesh *mesh);
/* 4 floats per vertex (red, green, blue, palette position) or NULL. */
const float *mb3d_mesh_colors(const mb3d_mesh *mesh);
/* 3 vertex numbers per triangle. */
const uint32_t *mb3d_mesh_faces(const mb3d_mesh *mesh);
/* OBJ, PLY or STL by the extension; 0 or -1. */
int mb3d_mesh_save(const mb3d_mesh *mesh, const char *path);

#ifdef __cplusplus
}
#endif

#endif
