{
  description = "Chat app env";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-26.05";
    nixpkgs-unstable.url = "nixpkgs/nixos-unstable";

    surrealdb = {
      url = "github:surrealdb/surrealdb";
      flake = true;
    };
  };

  outputs = {
    nixpkgs,
    nixpkgs-unstable,
    surrealdb,
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
          surrealdb.packages.${system}.default
          mkcert
        ]
        ++ (with pkgs-unstable; [
          surrealist
        ]);
    };
  };
}
