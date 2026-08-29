import { Extension, ExtensionContext } from 'shared';

// The archive format requires a frontend entrypoint; this extension is API only and renders nothing.
class NetCerbonixSsoTicketsExtension extends Extension {
  public initialize(_ctx: ExtensionContext): void {}
}

export default new NetCerbonixSsoTicketsExtension();
