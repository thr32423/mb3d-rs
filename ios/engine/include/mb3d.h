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

#ifdef __cplusplus
}
#endif

#endif
