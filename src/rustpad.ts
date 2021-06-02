/** Options passed in to the Rustpad constructor. */
type RustpadOptions = {
  readonly uri: string;
  readonly onConnected?: () => unknown;
  readonly onDisconnected?: () => unknown;
  readonly reconnectInterval?: number;
};

/** Browser client for Rustpad. */
class Rustpad {
  private ws?: WebSocket;
  private connecting?: boolean;
  private readonly intervalId: number;

  constructor(readonly options: RustpadOptions) {
    this.tryConnect();
    this.intervalId = window.setInterval(
      () => this.tryConnect(),
      options.reconnectInterval ?? 1000
    );
  }

  /** Destroy this Rustpad instance and close any sockets. */
  dispose() {
    window.clearInterval(this.intervalId);
    if (this.ws) this.ws.close();
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
    };
    ws.onclose = () => {
      if (this.ws) {
        this.ws = undefined;
        this.options.onDisconnected?.();
      } else {
        this.connecting = false;
      }
    };
  }
}

export default Rustpad;
