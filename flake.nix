{
  description = "Flake rust environment";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-26.05";
    nixpkgs-unstable.url = "nixpkgs/nixos-unstable";
  };

  outputs = {
    nixpkgs,
    nixpkgs-unstable,
    ...
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      config.allowUnfree = true;
    };
    pkgs-unstable = import nixpkgs-unstable {
      inherit system;
      config.allowUnfree = true;
    };
  in {
    devShells.${system}.default = pkgs.mkShell {
      packages = with pkgs;
        [
          cargo
          rustc
          rustup
          rustfmt
          openssl
          pkg-config
          websocat
        ]
        ++ (with pkgs-unstable; [
          surrealist
          surrealdb
        ]);
    };
  };
}
