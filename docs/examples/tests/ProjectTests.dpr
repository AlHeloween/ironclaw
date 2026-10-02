program ProjectTests;

{$APPTYPE CONSOLE}

uses
  TestFramework,
  TextTestRunner,
  TestCore in 'TestCore.pas',
  TestServices in 'TestServices.pas';

begin
  TextTestRunner.RunRegisteredTests;
end.

