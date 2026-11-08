object BenchFrm: TBenchFrm
  Left = 260
  Top = 200
  BorderStyle = bsDialog
  Caption = '@MB:Benchmark2'
  ClientHeight = 300
  ClientWidth = 520
  Color = clBtnFace
  Font.Charset = DEFAULT_CHARSET
  Font.Color = clWindowText
  Font.Height = -11
  Font.Name = 'MS Sans Serif'
  Font.Style = []
  OldCreateOrder = False
  Position = poScreenCenter
  OnShow = FormShow
  PixelsPerInch = 96
  TextHeight = 13
  object NoteLabel: TLabel
    Left = 12
    Top = 12
    Width = 496
    Height = 48
    AutoSize = False
    WordWrap = True
    Caption = '@MB:RendersTheFixedBenchmarkSceneInteger'
  end
  object RunBtn: TButton
    Left = 12
    Top = 68
    Width = 160
    Height = 25
    Caption = '@MB:RunTheBenchmark'
    TabOrder = 0
    OnClick = RunBtnClick
  end
  object CopyBtn: TButton
    Left = 180
    Top = 68
    Width = 140
    Height = 25
    Caption = '@MB:CopyTheResult'
    Enabled = False
    TabOrder = 1
    OnClick = CopyBtnClick
  end
  object StatusLabel: TLabel
    Left = 330
    Top = 74
    Width = 178
    Height = 13
    AutoSize = False
    Caption = ''
  end
  object ResultMemo: TMemo
    Left = 12
    Top = 104
    Width = 496
    Height = 150
    ReadOnly = True
    ScrollBars = ssVertical
    TabOrder = 2
  end
  object CloseBtn: TButton
    Left = 418
    Top = 264
    Width = 90
    Height = 26
    Caption = '@MB:Close'
    TabOrder = 3
    OnClick = CloseBtnClick
  end
  object Timer1: TTimer
    Enabled = False
    Interval = 200
    OnTimer = Timer1Timer
    Left = 460
    Top = 20
  end
end
