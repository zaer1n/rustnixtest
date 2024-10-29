{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    devenv.url = "github:cachix/devenv";
  };

  outputs =
    {
      self,
      nixpkgs,
      devenv,
      ...
    }@inputs:
    let
      forEachSystem =
        function:
        nixpkgs.lib.genAttrs [ "x86_64-linux" ] (system: function nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forEachSystem (pkgs: {
        default = devenv.lib.mkShell {
          inherit inputs pkgs;
          modules = [
            {
              # https://devenv.sh/reference/options/
              dotenv.disableHint = true;

              env = {
                LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
              };

              packages = with pkgs; [
                cargo-edit
                llvmPackages.clang
                llvmPackages.libclang
                pkg-config
                cmake
                ffmpeg
                zlib
              ];

              languages.rust.enable = true;
            }
          ];
        };
      });

      packages = forEachSystem (pkgs: { });
    };
}
