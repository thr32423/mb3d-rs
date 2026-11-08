# Five wishes of the Mandelbulb Maniacs group

The Facebook group *Mandelbulb Maniacs* (about 20 000 members) has used MB3D since 2013. Its posts were read for what people ask for, what trips them up and what they do by hand. These five changes answer the most common small wishes:

1. [360 photos that Facebook recognises](#1-360-photos-that-facebook-recognises)
2. [Maps of any name](#2-maps-of-any-name)
3. [A better colour picker](#3-a-better-colour-picker)
4. [Errors in plain words](#4-errors-in-plain-words)
5. [The Share window](#5-the-share-window)

The pictures on this page come from the program itself (`cargo run --example communityshots`).

## 1. 360 photos that Facebook recognises

**What people did by hand.** A spherical panorama only shows as an interactive 360 photo on Facebook when the file says it comes from a 360 camera. Members open each picture's file properties in Windows Explorer and type *RICOH* as the camera maker and *RICOH THETA S* as the model.

**What the program does now.** Every 2:1 picture rendered with the camera optic *spherical panorama* gets two sets of tags:

* the Photo Sphere tags (`GPano`, read by Facebook, Google Photos and most 360 viewers), which JPEG files already had;
* the camera of a 360 camera: Make `RICOH`, Model `RICOH THETA S`, Software `mb3d-rs`.

Both go into JPEG and PNG files, however they are saved:

* the main window's PNG, JPEG and 16 bit PNG buttons;
* the Monte Carlo window;
* the batch;
* the command line, including `mb3d vr360` on an existing JPEG;
* the Share window (below).

A panorama saved as JPEG, read back with `exiv2`:

```text
Exif.Image.Make            Ascii   6  RICOH
Exif.Image.Model           Ascii  14  RICOH THETA S
Exif.Image.Software        Ascii  14  mb3d-rs 0.3.4
Xmp.GPano.ProjectionType   XmpText 15 equirectangular
Xmp.GPano.UsePanoramaViewer XmpText 4 True
```

**Switching it off.** Untick *360 photos: the camera tag Facebook looks for* in the Share window. The program remembers the choice as `Pano360CameraTag` in its ini file. On the command line, set `MB3D_360_CAMERA_TAG=0`. The Photo Sphere tags stay either way.

## 2. Maps of any name

**What tripped people up.** Parameters refer to maps by number, so a picture had to be renamed to a number (`57.png`) before it could be a map. Beginners also found that a picture from another folder only worked until the program was closed.

**What the program does now.** Right-click a map number and choose any picture from any folder. These places have the map chooser:

* the Lighting window's light map number;
* the Lighting window's colour map number (*Object* tab, *Use a map for the diffuse color*);
* the *Map nr* of a height map formula in the Formula window. Its right-click chooser is new.

The picture becomes a map like this:

* A picture that already has a free number at the start of its name keeps it.
* Any other picture is copied into the maps folder as `<number> <its name>`, with the next free number. `Brick wall (rough).png` becomes `3052 Brick wall (rough).png`.
* A picture that is already in the map folders under a number keeps that number, and nothing is copied twice.
* The copy never replaces a file.

The main window's messages say where the copy went:

```text
Brick wall (rough).png was copied into the maps folder as map 3052: …/M3Maps/3052 Brick wall (rough).png
```

Because the copy has a number at the start of its name, MB3D finds it too.

A background picture from another folder is copied into the maps folder under its own name, so the scene finds it the next time it is opened.

<a href="img/community/maps_light_window.png"><img src="img/community/maps_light_window.png" width="235" alt="The Lighting window with a brick picture as the colour map, number 3052"></a>
<a href="img/community/maps_main.png"><img src="img/community/maps_main.png" width="520" alt="The fractal coloured by the brick map"></a>

The hint of the map number says how it works. In the Formula window it shows on the map number of a height map formula:

<a href="img/community/maps_formula_window.png"><img src="img/community/maps_formula_window.png" width="321" alt="The Formula window with HeightMapLimIFS and the hint of its map number"></a>

**A bug that went away.** The old chooser silently did nothing for a picture without a number. It also took the first digits anywhere in the name, so `brick2.png` loaded map 2, a different picture.

## 3. A better colour picker

The colour dialog opens from every colour button: lights, the palette, ground, clouds and others. It has three additions.

**Hex.** The colour is shown as `#RRGGBB`, ready to copy. You can also type one: `#4080C0`, `4080C0`, `0x4080C0` and the short `#48C` all work. A moderator of the group asked for this.

**Recent colours.** The eight colours chosen last sit under the basic colours, newest first. The program keeps them between sessions.

**Eyedropper.** Press *Eyedropper*, then click any pixel in any of the program's windows: the picture, a map preview, the palette or another window. That pixel's colour becomes the dialog's colour. The cursor is a cross while it waits, and Esc stops it.

The eyedropper reads the window as the program paints it, so it works at any interface size. It cannot read the windows of other programs, because Wayland, the Linux desktop protocol, does not allow that.

<a href="img/community/eyedropper_on.png"><img src="img/community/eyedropper_on.png" width="300" alt="The colour dialog waiting for the eyedropper's click"></a>
<a href="img/community/eyedropper_picked.png"><img src="img/community/eyedropper_picked.png" width="300" alt="The colour taken from the fractal: #236261"></a>

Left: waiting for the click. Right: a colour taken from the fractal in the main window.

**Borrowing a palette.** Members often keep another artist's colours and change everything else. The *Color adjustment* window has three new buttons for this:

* **Colors from file...** takes only the colours from a parameter file (`.m3p`, `.m3i`, text) or a light file (`.m3l`). It copies the surface palette with its transparency and the inside colours. The lights, fog, maps, fractal and view stay as they are.
* **From Clipboard** does the same with parameters copied from a post.
* **Save colors...** writes the lighting with these colours as a light file, which MB3D can open too.

<a href="img/community/color_form_buttons.png"><img src="img/community/color_form_buttons.png" width="760" alt="The Color adjustment window with Colors from file, From Clipboard and Save colors"></a>

## 4. Errors in plain words

The group's beginners keep asking what two messages mean: "formula option is not valid" and "mixing of dIFS will not work". These errors now open a window that explains them and says what to do.

**A formula that is not installed.** Parameters can name a formula this computer does not have. That often happens with parameters from a tutorial, because each installation has different formula packs. Opening such parameters used to end with a terse line. Pasting them said "No text parameters found", which was wrong.

The window now says:

* which formula is missing;
* that formulas are `.m3f` files, often shared with the parameters or in a formula pack;
* which folders formulas go into;
* which installed formulas have a similar name.

<a href="img/community/explain_missing_formula.png"><img src="img/community/explain_missing_formula.png" width="610" alt="Formula not installed: SphereTrex, with the formula folders and SphereTree as a similar name"></a>

**dIFS formulas mixed with others.** A dIFS formula builds a shape straight from distances. Bulbs and boxes are fractals iterated step by step. One formula chain cannot calculate both, so the calculation stops with "mixing of dIFS with other formulas will not work".

The window explains the two kinds and the DE combination that shows both. When the slots hold one kind first and the other after it, the window offers **Set up DE combination**. That button joins the two parts with *min*, which keeps both objects, and calculates again.

<a href="img/community/explain_difs.png"><img src="img/community/explain_difs.png" width="610" alt="Why the calculation cannot start: dIFS formulas mixed with others, with the button Set up DE combination"></a>

After the button, the Formula window is on its *DEcombinate* tab with part 2 starting at slot 3, and the picture is calculated:

<a href="img/community/explain_difs_fixed_formulas.png"><img src="img/community/explain_difs_fixed_formulas.png" width="321" alt="The Formula window on DEcombinate, Min, part 2 at slot 3"></a>
<a href="img/community/explain_difs_fixed_main.png"><img src="img/community/explain_difs_fixed_main.png" width="520" alt="The main window with the calculated picture"></a>

When the two kinds take turns in the slots, the window says how to reorder them first. It does not offer the button then.

**The other cases** get a short explanation too:

* one part of a DE combination mixes dIFS and other formulas;
* dIFS transforms have no dIFS shape after them;
* dIFS formulas are in an interpolation hybrid;
* no slot holds a fractal formula with iterations, which is the "formula option is not valid" case.

## 5. The Share window

**What the group asks of a post.**

* The parameters come as a `.txt` file. Facebook turns pasted parameters into links, and it has banned members for "spam" because of them.
* A tweak credits the artist whose parameters it started from.

**What the window does.** The share icon in the main window's title bar, left of the log icon, opens it:

<a href="img/community/titlebar_share_hint.png"><img src="img/community/titlebar_share_hint.png" width="489" alt="The share icon in the title bar with its hint"></a>

<a href="img/community/share_window_made.png"><img src="img/community/share_window_made.png" width="540" alt="The Share window after Make the files"></a>

The window starts filled in from the scene:

* **Title** is the parameters' name.
* **Artist** and **Credit** come from MB3D's *Orig* and *Mod* authors on the main window's *Infos* page. When the parameters were changed by someone else, the credit reads "Tweak of parameters by …" with the original artist.
* **Picture (JPEG)** comes in two sizes. *Facebook size* has a long side of 2048 pixels, which Facebook keeps sharp; a 360 photo stays up to 6000 pixels wide. *Full size* keeps the picture as it is.
* **Parameters as a text file** writes the parameters with the title, artist, credit, formulas and program version above them. The file opens again in mb3d-rs and in MB3D, because both look for the `Mandelbulb3Dv18{` block.
* **Post text** is written from the fields and can be edited before it is used. It names the formulas; a DE combination is shown as both parts and its kind.

**Make the files** writes a folder of its own, `Pictures/Share/<title>` by default:

```text
Blue Bulb/
  Blue Bulb.jpg    the picture, with the 360 tags for a panorama
  Blue Bulb.txt    the parameters with the lines about them
  post.txt         the post text
```

It also puts the post text on the clipboard. **Open the folder** shows the folder in the file manager. A second share with the same title gets its own folder, `Blue Bulb 2`.

The post text of the example:

```text
Blue Bulb by Sebastian
Tweak of parameters by Marije
Formulas: Integer Power
Made with mb3d-rs 0.3.4 (Mandelbulb3D for Windows, Linux and macOS)
Parameters: Blue Bulb.txt (attached)
```

## Two small fixes found on the way

* The light box behind a title-bar icon under the mouse used to be painted solid white, so the History and Log icons disappeared while the mouse was on them. The box is light again, and the icon shows through it.
* A hint wider than a narrow window, such as the Lighting or Formula window, was cut off. Such a hint now wraps at the window's width.
