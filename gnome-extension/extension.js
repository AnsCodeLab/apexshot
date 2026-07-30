// SPDX-License-Identifier: AGPL-3.0-or-later

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

import {PreviewStacker} from './preview-stacking.js';
import {ShellOverlayService} from './shell-overlay.js';
import {WindowListService} from './window-list.js';

export default class ApexShotExtension extends Extension {
    enable() {
        this._previewStacker = new PreviewStacker();
        this._previewStacker.enable();

        this._shellOverlay = new ShellOverlayService();
        this._shellOverlay.enable();

        this._windowList = new WindowListService();
        this._windowList.enable();
    }

    disable() {
        this._windowList.disable();
        this._windowList = null;

        this._shellOverlay.disable();
        this._shellOverlay = null;

        this._previewStacker.disable();
        this._previewStacker = null;
    }
}
