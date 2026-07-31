// SPDX-License-Identifier: AGPL-3.0-or-later

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Clutter from 'gi://Clutter';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const DBUS_NAME = 'org.apexshot.ShellOverlay';
const DBUS_PATH = '/org/apexshot/ShellOverlay';

const DBUS_INTERFACE = `
<node>
  <interface name="org.apexshot.ShellOverlay">
    <method name="ShowMask">
      <arg type="i" name="x" direction="in"/>
      <arg type="i" name="y" direction="in"/>
      <arg type="i" name="width" direction="in"/>
      <arg type="i" name="height" direction="in"/>
    </method>
    <method name="HideMask"/>
    <method name="ShowCountdown">
      <arg type="i" name="x" direction="in"/>
      <arg type="i" name="y" direction="in"/>
      <arg type="i" name="width" direction="in"/>
      <arg type="i" name="height" direction="in"/>
      <arg type="u" name="seconds" direction="in"/>
    </method>
    <method name="HideCountdown"/>
  </interface>
</node>`;

const MASK_STYLE = 'background-color: rgba(0, 0, 0, 0.55);';
const COUNTDOWN_SIZE = 184;
const COUNTDOWN_STYLE = 'background-color: rgba(0, 0, 0, 0.94); border-radius: 92px;';
const COUNTDOWN_LABEL_STYLE = 'color: white; font-size: 72px; font-weight: bold;';

/// Dims everything outside the area ApexShot is recording.
///
/// The mask is four plain widgets (above, left, right, below the capture
/// rect) parented to `global.window_group`, so it dims windows without
/// covering the shell chrome.
export class ShellOverlayService {
    constructor() {
        this._dbus = null;
        this._nameId = 0;
        this._monitorsChangedId = 0;
        this._maskGroup = null;
        this._rect = null;
        this._countdown = null;
        this._countdownTimerId = 0;
    }

    enable() {
        this._dbus = Gio.DBusExportedObject.wrapJSObject(DBUS_INTERFACE, this);
        this._dbus.export(Gio.DBus.session, DBUS_PATH);

        this._nameId = Gio.DBus.session.own_name(
            DBUS_NAME,
            Gio.BusNameOwnerFlags.REPLACE,
            null,
            null);

        this._monitorsChangedId = Main.layoutManager.connect('monitors-changed',
            () => this._redraw());
    }

    disable() {
        if (this._monitorsChangedId) {
            Main.layoutManager.disconnect(this._monitorsChangedId);
            this._monitorsChangedId = 0;
        }

        this._destroyMask();
        this._destroyCountdown();

        if (this._nameId) {
            Gio.DBus.session.unown_name(this._nameId);
            this._nameId = 0;
        }

        if (this._dbus) {
            this._dbus.unexport();
            this._dbus = null;
        }
    }

    ShowMask(x, y, width, height) {
        if (width <= 0 || height <= 0) {
            this.HideMask();
            return;
        }

        this._rect = {x, y, width, height};
        this._redraw();
    }

    HideMask() {
        this._rect = null;
        this._destroyMask();
    }

    ShowCountdown(x, y, width, height, seconds) {
        this._destroyCountdown();
        if (width <= 0 || height <= 0 || seconds <= 0)
            return;

        let remaining = seconds;
        const label = new St.Label({
            text: `${remaining}`,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
            style: COUNTDOWN_LABEL_STYLE,
        });
        this._countdown = new St.Bin({
            reactive: false,
            x: Math.round(x + width / 2 - COUNTDOWN_SIZE / 2),
            y: Math.round(y + height / 2 - COUNTDOWN_SIZE / 2),
            width: COUNTDOWN_SIZE,
            height: COUNTDOWN_SIZE,
            style: COUNTDOWN_STYLE,
        });
        this._countdown.set_child(label);
        global.window_group.add_child(this._countdown);

        this._countdownTimerId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 1, () => {
            remaining--;
            if (remaining <= 0) {
                this._countdownTimerId = 0;
                this._destroyCountdown();
                return GLib.SOURCE_REMOVE;
            }
            label.text = `${remaining}`;
            return GLib.SOURCE_CONTINUE;
        });
    }

    HideCountdown() {
        this._destroyCountdown();
    }

    _redraw() {
        if (!this._rect)
            return;

        const {x, y, width, height} = this._rect;
        const stageWidth = global.stage.width;
        const stageHeight = global.stage.height;

        const left = Math.max(0, Math.min(x, stageWidth));
        const top = Math.max(0, Math.min(y, stageHeight));
        const right = Math.max(left, Math.min(x + width, stageWidth));
        const bottom = Math.max(top, Math.min(y + height, stageHeight));

        if (!this._maskGroup) {
            this._maskGroup = new St.Widget({reactive: false});
            global.window_group.add_child(this._maskGroup);
        }

        this._maskGroup.remove_all_children();
        this._maskGroup.set_position(0, 0);
        this._maskGroup.set_size(stageWidth, stageHeight);

        const bands = [
            [0, 0, stageWidth, top],
            [0, top, left, bottom - top],
            [right, top, stageWidth - right, bottom - top],
            [0, bottom, stageWidth, stageHeight - bottom],
        ];

        for (const [bandX, bandY, bandWidth, bandHeight] of bands) {
            if (bandWidth <= 0 || bandHeight <= 0)
                continue;

            this._maskGroup.add_child(new St.Widget({
                reactive: false,
                x: bandX,
                y: bandY,
                width: bandWidth,
                height: bandHeight,
                style: MASK_STYLE,
            }));
        }
    }

    _destroyMask() {
        if (!this._maskGroup)
            return;

        this._maskGroup.destroy();
        this._maskGroup = null;
    }

    _destroyCountdown() {
        if (this._countdownTimerId) {
            GLib.source_remove(this._countdownTimerId);
            this._countdownTimerId = 0;
        }
        if (this._countdown) {
            this._countdown.destroy();
            this._countdown = null;
        }
    }
}
