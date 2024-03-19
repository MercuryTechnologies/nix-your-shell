{
  lib,
  newScope,
  inputs,
}:
lib.makeScope newScope (
  self:
    {inherit inputs;}
    // (lib.packagesFromDirectoryRecursive {
      inherit (self) callPackage;
      directory = ./packages;
    })
)
