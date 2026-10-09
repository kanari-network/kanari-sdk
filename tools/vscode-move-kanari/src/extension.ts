// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

import * as vscode from 'vscode';
import { log } from './log';

/** Information related to this extension itself, such as its identifier and version. */
export class Extension {
  /** The string used to uniquely identify this particular extension to VS Code. */
  readonly identifier = 'kanari-network.move-kanari';

  private readonly extension: vscode.Extension<unknown> | undefined;

  constructor() {
    const extension = vscode.extensions.getExtension(this.identifier);
    if (extension === undefined) {
      log.info(`extension ${this.identifier} is not available; using fallback version`);
    }
    this.extension = extension;
  }

  /** The version string. */
  get version(): string {
    if (this.extension === undefined) {
      return 'unknown';
    }
    for (const entry of Object.entries(this.extension.packageJSON)) {
      if (entry[0] === 'version') {
        return entry[1] as string;
      }
    }
    return 'unknown';
  }
}
