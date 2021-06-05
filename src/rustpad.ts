import { OpSeq } from "rustpad-wasm";
import type {
  editor,
  IDisposable,
} from "monaco-editor/esm/vs/editor/editor.api";

/** Options passed in to the Rustpad constructor. */
export type RustpadOptions = {
  readonly uri: string;
  readonly editor: editor.IStandaloneCodeEditor;
  readonly onConnected?: () => unknown;
  readonly onDisconnected?: () => unknown;
  readonly onDesynchronized?: () => unknown;
  readonly onChangeLanguage?: (language: string) => unknown;
  readonly onChangeUsers?: (users: Record<number, UserInfo>) => unknown;
  readonly reconnectInterval?: number;
};

/** A user currently editing the document. */
export type UserInfo = {
  readonly name: string;
  readonly hue: number;
};

/** Browser client for Rustpad. */
class Rustpad {
  private ws?: WebSocket;
  private connecting?: boolean;
  private recentFailures: number = 0;
  private readonly model: editor.ITextModel;
  private readonly onChangeHandle: IDisposable;
  private readonly onCursorHandle: IDisposable;
  private readonly onSelectionHandle: IDisposable;
  private readonly beforeUnload: (event: BeforeUnloadEvent) => void;
  private readonly tryConnectId: number;
  private readonly resetFailuresId: number;

  // Client-server state
  private me: number = -1;
  private revision: number = 0;
  private outstanding?: OpSeq;
  private buffer?: OpSeq;
  private users: Record<number, UserInfo> = {};
  private userCursors: Record<number, CursorData> = {};
  private myInfo?: UserInfo;
  private cursorData: CursorData = { cursors: [], selections: [] };

  // Intermittent local editor state
  private lastValue: string = "";
  private ignoreChanges: boolean = false;
  private oldDecorations: string[] = [];

  constructor(readonly options: RustpadOptions) {
    this.model = options.editor.getModel()!;
    this.onChangeHandle = options.editor.onDidChangeModelContent((e) =>
      this.onChange(e)
    );
    this.onCursorHandle = options.editor.onDidChangeCursorPosition((e) =>
      this.onCursor(e)
    );
    this.onSelectionHandle = options.editor.onDidChangeCursorSelection((e) =>
      this.onSelection(e)
    );
    this.beforeUnload = (event: BeforeUnloadEvent) => {
      if (this.outstanding) {
        event.preventDefault();
        event.returnValue = "";
      } else {
        delete event.returnValue;
      }
    };
    window.addEventListener("beforeunload", this.beforeUnload);

    const interval = options.reconnectInterval ?? 1000;
    this.tryConnect();
    this.tryConnectId = window.setInterval(() => this.tryConnect(), interval);
    this.resetFailuresId = window.setInterval(
      () => (this.recentFailures = 0),
      15 * interval
    );
  }

  /** Destroy this Rustpad instance and close any sockets. */
  dispose() {
    window.clearInterval(this.tryConnectId);
    window.clearInterval(this.resetFailuresId);
    this.onSelectionHandle.dispose();
    this.onCursorHandle.dispose();
    this.onChangeHandle.dispose();
    window.removeEventListener("beforeunload", this.beforeUnload);
    this.ws?.close();
  }

  /** Try to set the language of the editor, if connected. */
  setLanguage(language: string): boolean {
    this.ws?.send(`{"SetLanguage":${JSON.stringify(language)}}`);
    return this.ws !== undefined;
  }

  /** Set the user's information. */
  setInfo(info: UserInfo) {
    this.myInfo = info;
    this.sendInfo();
  }

  /**
   * Attempts a WebSocket connection.
   *
   * Safety Invariant: Until this WebSocket connection is closed, no other
   * connections will be attempted because either `this.ws` or
   * `this.connecting` will be set to a truthy value.
   *
   * Liveness Invariant: After this WebSocket connection closes, either through
   * error or successful end, both `this.connecting` and `this.ws` will be set
   * to falsy values.
   */
  private tryConnect() {
    if (this.connecting || this.ws) return;
    this.connecting = true;
    const ws = new WebSocket(this.options.uri);
    ws.onopen = () => {
      this.connecting = false;
      this.ws = ws;
      this.options.onConnected?.();
      this.users = {};
      this.options.onChangeUsers?.(this.users);
      this.sendInfo();
      this.sendCursorData();
      if (this.outstanding) {
        this.sendOperation(this.outstanding);
      }
    };
    ws.onclose = () => {
      if (this.ws) {
        this.ws = undefined;
        this.options.onDisconnected?.();
        if (++this.recentFailures >= 5) {
          // If we disconnect 5 times within 15 reconnection intervals, then the
          // client is likely desynchronized and needs to refresh.
          this.dispose();
          this.options.onDesynchronized?.();
        }
      } else {
        this.connecting = false;
      }
    };
    ws.onmessage = ({ data }) => {
      if (typeof data === "string") {
        this.handleMessage(JSON.parse(data));
      }
    };
  }

  private handleMessage(msg: ServerMsg) {
    if (msg.Identity !== undefined) {
      this.me = msg.Identity;
    } else if (msg.History !== undefined) {
      const { start, operations } = msg.History;
      if (start > this.revision) {
        console.warn("History message has start greater than last operation.");
        this.ws?.close();
        return;
      }
      for (let i = this.revision - start; i < operations.length; i++) {
        let { id, operation } = operations[i];
        this.revision++;
        if (id === this.me) {
          this.serverAck();
        } else {
          operation = OpSeq.from_str(JSON.stringify(operation));
          this.applyServer(operation);
        }
      }
    } else if (msg.Language !== undefined) {
      this.options.onChangeLanguage?.(msg.Language);
    } else if (msg.UserInfo !== undefined) {
      const { id, info } = msg.UserInfo;
      if (id !== this.me) {
        this.users = { ...this.users };
        if (info) {
          this.users[id] = info;
        } else {
          delete this.users[id];
          delete this.userCursors[id];
        }
        this.updateCursors();
        this.options.onChangeUsers?.(this.users);
      }
    } else if (msg.UserCursor !== undefined) {
      const { id, data } = msg.UserCursor;
      if (id !== this.me) {
        this.userCursors[id] = data;
        this.updateCursors();
      }
    }
  }

  private serverAck() {
    if (!this.outstanding) {
      console.warn("Received serverAck with no outstanding operation.");
      return;
    }
    this.outstanding = this.buffer;
    this.buffer = undefined;
    if (this.outstanding) {
      this.sendOperation(this.outstanding);
    }
  }

  private applyServer(operation: OpSeq) {
    if (this.outstanding) {
      const pair = this.outstanding.transform(operation)!;
      this.outstanding = pair.first();
      operation = pair.second();
      if (this.buffer) {
        const pair = this.buffer.transform(operation)!;
        this.buffer = pair.first();
        operation = pair.second();
      }
    }
    this.applyOperation(operation);
  }

  private applyClient(operation: OpSeq) {
    if (!this.outstanding) {
      this.sendOperation(operation);
      this.outstanding = operation;
    } else if (!this.buffer) {
      this.buffer = operation;
    } else {
      this.buffer = this.buffer.compose(operation);
    }
    this.transformCursors(operation);
  }

  private sendOperation(operation: OpSeq) {
    const op = operation.to_string();
    this.ws?.send(`{"Edit":{"revision":${this.revision},"operation":${op}}}`);
  }

  private sendInfo() {
    if (this.myInfo) {
      this.ws?.send(`{"ClientInfo":${JSON.stringify(this.myInfo)}}`);
    }
  }

  private sendCursorData() {
    if (!this.buffer) {
      this.ws?.send(`{"CursorData":${JSON.stringify(this.cursorData)}}`);
    }
  }

  private applyOperation(operation: OpSeq) {
    if (operation.is_noop()) return;

    this.ignoreChanges = true;
    const ops: (string | number)[] = JSON.parse(operation.to_string());
    let index = 0;

    for (const op of ops) {
      if (typeof op === "string") {
        // Insert
        const pos = this.model.getPositionAt(index);
        index += op.length;
        this.model.pushEditOperations(
          this.options.editor.getSelections(),
          [
            {
              range: {
                startLineNumber: pos.lineNumber,
                startColumn: pos.column,
                endLineNumber: pos.lineNumber,
                endColumn: pos.column,
              },
              text: op,
              forceMoveMarkers: true,
            },
          ],
          () => null
        );
      } else if (op >= 0) {
        // Retain
        index += op;
      } else {
        // Delete
        const chars = -op;
        var from = this.model.getPositionAt(index);
        var to = this.model.getPositionAt(index + chars);
        this.model.pushEditOperations(
          this.options.editor.getSelections(),
          [
            {
              range: {
                startLineNumber: from.lineNumber,
                startColumn: from.column,
                endLineNumber: to.lineNumber,
                endColumn: to.column,
              },
              text: "",
              forceMoveMarkers: true,
            },
          ],
          () => null
        );
      }
    }

    this.lastValue = this.model.getValue();
    this.ignoreChanges = false;

    this.transformCursors(operation);
  }

  private transformCursors(operation: OpSeq) {
    for (const data of Object.values(this.userCursors)) {
      data.cursors = data.cursors.map((c) => operation.transform_index(c));
      data.selections = data.selections.map(([s, e]) => [
        operation.transform_index(s),
        operation.transform_index(e),
      ]);
    }
    this.updateCursors();
  }

  private updateCursors() {
    const decorations: editor.IModelDeltaDecoration[] = [];

    for (const [id, data] of Object.entries(this.userCursors)) {
      if (id in this.users) {
        const { hue, name } = this.users[id as any];
        generateCssStyles(hue);

        for (const cursor of data.cursors) {
          const position = this.model.getPositionAt(cursor);
          decorations.push({
            options: {
              className: `remote-cursor-${hue}`,
              stickiness: 1,
              zIndex: 2,
            },
            range: {
              startLineNumber: position.lineNumber,
              startColumn: position.column,
              endLineNumber: position.lineNumber,
              endColumn: position.column,
            },
          });
        }
        for (const selection of data.selections) {
          const position = this.model.getPositionAt(selection[0]);
          const positionEnd = this.model.getPositionAt(selection[1]);
          decorations.push({
            options: {
              className: `remote-selection-${hue}`,
              hoverMessage: {
                value: name,
              },
              stickiness: 1,
              zIndex: 1,
            },
            range: {
              startLineNumber: position.lineNumber,
              startColumn: position.column,
              endLineNumber: positionEnd.lineNumber,
              endColumn: positionEnd.column,
            },
          });
        }
      }
    }

    this.oldDecorations = this.model.deltaDecorations(
      this.oldDecorations,
      decorations
    );
  }

  private onChange(event: editor.IModelContentChangedEvent) {
    if (!this.ignoreChanges) {
      const content = this.lastValue;
      let offset = 0;

      let operation = OpSeq.new();
      operation.retain(content.length);
      event.changes.sort((a, b) => b.rangeOffset - a.rangeOffset);
      for (const change of event.changes) {
        const { text, rangeOffset, rangeLength } = change;
        const restLength = content.length + offset - rangeOffset - rangeLength;
        const changeOp = OpSeq.new();
        changeOp.retain(rangeOffset);
        changeOp.delete(rangeLength);
        changeOp.insert(text);
        changeOp.retain(restLength);
        operation = operation.compose(changeOp)!;
        offset += changeOp.target_len() - changeOp.base_len();
      }
      this.applyClient(operation);
      this.lastValue = this.model.getValue();
    }
  }

  private onCursor(event: editor.ICursorPositionChangedEvent) {
    const cursors = [event.position, ...event.secondaryPositions];
    this.cursorData.cursors = cursors.map((p) => this.model.getOffsetAt(p));
    this.sendCursorData();
  }

  private onSelection(event: editor.ICursorSelectionChangedEvent) {
    const selections = [event.selection, ...event.secondarySelections];
    this.cursorData.selections = selections.map((s) => [
      this.model.getOffsetAt(s.getStartPosition()),
      this.model.getOffsetAt(s.getEndPosition()),
    ]);
    this.sendCursorData();
  }
}

type UserOperation = {
  id: number;
  operation: any;
};

type CursorData = {
  cursors: number[];
  selections: [number, number][];
};

type ServerMsg = {
  Identity?: number;
  History?: {
    start: number;
    operations: UserOperation[];
  };
  Language?: string;
  UserInfo?: {
    id: number;
    info: UserInfo | null;
  };
  UserCursor?: {
    id: number;
    data: CursorData;
  };
};

/** Cache for private use by `generateCssStyles()`. */
const generatedStyles = new Set<number>();

/** Add CSS styles for a remote user's cursor and selection. */
function generateCssStyles(hue: number) {
  if (!generatedStyles.has(hue)) {
    generatedStyles.add(hue);
    const css = `
      .monaco-editor .remote-selection-${hue} {
        background-color: hsla(${hue}, 90%, 80%, 0.5);
      }
      .monaco-editor .remote-cursor-${hue} {
        border-left: 2px solid hsl(${hue}, 90%, 25%);
      }
    `;
    const element = document.createElement("style");
    const text = document.createTextNode(css);
    element.appendChild(text);
    document.head.appendChild(element);
  }
}

export default Rustpad;
