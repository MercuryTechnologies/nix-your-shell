{lib}: pkgs:
lib.pipe pkgs [
  (builtins.map (
    pkg:
      lib.pipe pkg.checks [
        builtins.attrValues
        (builtins.map
          (
            check:
              lib.nameValuePair
              # The Nix CLI doesn't like attribute names that contain dots.
              (builtins.replaceStrings ["."] ["-"] check.pname)
              check
          ))
      ]
  ))
  lib.flatten
  builtins.listToAttrs
]
