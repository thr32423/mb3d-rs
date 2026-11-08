object DrosteFrm: TDrosteFrm
  Left = 220
  Top = 160
  BorderStyle = bsDialog
  Caption = '@MB:DrosteEffect'
  ClientHeight = 432
  ClientWidth = 764
  Color = clBtnFace
  Font.Charset = DEFAULT_CHARSET
  Font.Color = clWindowText
  Font.Height = -11
  Font.Name = 'MS Sans Serif'
  Font.Style = []
  OldCreateOrder = False
  Position = poScreenCenter
  OnShow = FormShow
  OnHide = FormHide
  PixelsPerInch = 96
  TextHeight = 13
  object Image1: TImage
    Left = 8
    Top = 8
    Width = 500
    Height = 375
    Center = True
    OnMouseDown = Image1MouseDown
    OnMouseMove = Image1MouseMove
    OnMouseUp = Image1MouseUp
    OnMouseWheel = Image1MouseWheel
  end
  object OnCheck: TCheckBox
    Left = 520
    Top = 8
    Width = 236
    Height = 30
    Hint = '@MB:ThePictureRepeatedInItself'
    Caption = '@MB:RepeatThePictureInItself'
    ParentShowHint = False
    ShowHint = True
    WordWrap = True
    TabOrder = 0
    OnClick = OnCheckClick
  end
  object CxLabel: TLabel
    Left = 520
    Top = 50
    Width = 120
    Height = 13
    Caption = '@MB:CentreX'
  end
  object CxEdit: TEdit
    Left = 666
    Top = 46
    Width = 90
    Height = 21
    Hint = '@MB:TheCentreAsAPartOf'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 1
    OnChange = EditChange
  end
  object CyLabel: TLabel
    Left = 520
    Top = 76
    Width = 120
    Height = 13
    Caption = '@MB:CentreY'
  end
  object CyEdit: TEdit
    Left = 666
    Top = 72
    Width = 90
    Height = 21
    Hint = '@MB:TheCentreAsAPartOf'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 2
    OnChange = EditChange
  end
  object InnerLabel: TLabel
    Left = 520
    Top = 102
    Width = 120
    Height = 13
    Caption = '@MB:InnerRadius'
  end
  object InnerEdit: TEdit
    Left = 666
    Top = 98
    Width = 90
    Height = 21
    Hint = '@MB:TheRadiiAsPartsOfThe'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 3
    OnChange = EditChange
  end
  object OuterLabel: TLabel
    Left = 520
    Top = 128
    Width = 120
    Height = 13
    Caption = '@MB:OuterRadius'
  end
  object OuterEdit: TEdit
    Left = 666
    Top = 124
    Width = 90
    Height = 21
    Hint = '@MB:TheRadiiAsPartsOfThe'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 4
    OnChange = EditChange
  end
  object TwistLabel: TLabel
    Left = 520
    Top = 154
    Width = 120
    Height = 13
    Caption = '@MB:TwistTurns'
  end
  object TwistEdit: TEdit
    Left = 666
    Top = 150
    Width = 90
    Height = 21
    Hint = '@MB:TurnsOfTheSpiralPerStep'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 5
    OnChange = EditChange
  end
  object BlendLabel: TLabel
    Left = 520
    Top = 180
    Width = 120
    Height = 13
    Caption = '@MB:Crossfade'
  end
  object BlendEdit: TEdit
    Left = 666
    Top = 176
    Width = 90
    Height = 21
    Hint = '@MB:ThePartOfAStepOverWhich'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 6
    OnChange = EditChange
  end
  object PhaseLabel: TLabel
    Left = 520
    Top = 206
    Width = 120
    Height = 13
    Caption = '@MB:Phase'
  end
  object PhaseEdit: TEdit
    Left = 666
    Top = 202
    Width = 90
    Height = 21
    Hint = '@MB:ZoomsThroughOneShrinkStep'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 7
    OnChange = EditChange
  end
  object PhaseBar: TTrackBar
    Left = 518
    Top = 228
    Width = 240
    Height = 24
    Hint = '@MB:ZoomsThroughOneShrinkStep'
    Max = 100
    ParentShowHint = False
    ShowHint = True
    TabOrder = 8
    ThumbLength = 14
    TickStyle = tsNone
    OnChange = PhaseBarChange
  end
  object NoteLabel: TLabel
    Left = 520
    Top = 260
    Width = 236
    Height = 104
    AutoSize = False
    WordWrap = True
    Caption = '@MB:ClickOrDragOnThePreview'
  end
  object PaintBtn: TButton
    Left = 520
    Top = 372
    Width = 114
    Height = 25
    Hint = '@MB:PaintsThePictureAgainWithThe'
    Caption = '@MB:PaintNow'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 9
    OnClick = PaintBtnClick
  end
  object ResetBtn: TButton
    Left = 642
    Top = 372
    Width = 114
    Height = 25
    Hint = '@MB:TheSettingsBackToTheirStart'
    Caption = '@MB:Reset2'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 10
    OnClick = ResetBtnClick
  end
  object CloseBtn: TButton
    Left = 642
    Top = 402
    Width = 114
    Height = 25
    Caption = '@MB:Close'
    TabOrder = 11
    OnClick = CloseBtnClick
  end
  object StatusLabel: TLabel
    Left = 8
    Top = 390
    Width = 500
    Height = 40
    AutoSize = False
    WordWrap = True
    Caption = ''
  end
end
