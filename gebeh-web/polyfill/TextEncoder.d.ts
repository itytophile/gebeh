// to prevent tsc to reemit the file in dist/ and breaking everything
declare module "TextEncoder.js" {
  export function foo(): void;
}
