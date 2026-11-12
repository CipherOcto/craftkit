//! The **grid battery's registry**, pinned.
//!
//! # Why this test exists
//!
//! §869 converted this battery from one declaration per command to
//! **one table per file** — the `commands!` table of the derive. A
//! conversion that changed a single id, a label, a layer, an icon, a menu
//! path, a parameter or a flag would be a conversion that changed the
//! tree.
//!
//! So the tree's own registry is **pinned**: every command of this battery
//! is written down here — its id, its label, its area, its capability, its
//! layer, its icon, its documentation, its chord, its menu, its parameters
//! and its flags — and this test says whether the registry still says it.
//!
//! The snapshot was taken **before** the conversion, from the registry
//! itself. So the test is not a description of what the table says; it is
//! a description of **what the tree was**, and the table has to agree with
//! it.
//!
//! # Why it is here and not in the battery
//!
//! A battery's own test binary sees **no commands**: the plugin's items
//! are behind its anchor, and the anchor keeps them for a program that
//! links the plugin. The studio links the batteries, so the studio's
//! ecosystem is where the registry of a battery can be read.

#![cfg(feature = "grid")]

use craftkit_sdk::command::CommandSpec;

/// The commands of the grid battery, as the registry held them.
///
/// **The snapshot**: taken before the conversion, from the registry.
const EXPECTED: &[&str] = &[
    r#"grid.calc | Calc | grid | command | grid | list | The **values** of a sheet: every formula, computed. | - | grid | sheet(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.cell.align | Align | grid | command | grid | grid | **Align** a cell: how its text lies across it and how it lies down it, and how far it is indented and whether it wraps. | - | grid | sheet(node-ref)!,cell(text)!,horizontal(text)?,vertical(text)?,indent(number)?,wrap(bool)? | mutates=true | undoable=true"#,
    r#"grid.cell.border.clear | Clear | grid | command | grid | eraser | **Take away** the border of one edge of a cell. | - | grid | sheet(node-ref)!,cell(text)!,edge(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.border.get | Get | grid | command | grid | grid | **Read** the borders of a cell, or of the whole sheet. | - | grid | sheet(node-ref)!,cell(text)? | mutates=false | undoable=false"#,
    r#"grid.cell.border.set | Set | grid | command | grid | grid | **Set** the border of one edge of a cell. | - | grid | sheet(node-ref)!,cell(text)!,edge(text)!,style(text)!,colour(text)? | mutates=true | undoable=true"#,
    r#"grid.cell.checkbox.set | Set | grid | command | grid | check | **Make** a cell a checkbox. | - | grid | sheet(node-ref)!,cell(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.checkbox.toggle | Toggle | grid | command | grid | check | **Toggle** the checkbox of a cell. | - | grid | sheet(node-ref)!,cell(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.clear | Clear | grid | command | grid | minus | Empty a **cell**. | - | grid | sheet(node-ref)!,cell(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.clear.contents | Contents | grid | command | grid | eraser | **Clear the contents** of a cell: its value and its formula. | - | grid | sheet(node-ref)!,cell(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.clear.format | Format | grid | command | grid | eraser | **Clear the format** of a cell: its number format and its style. | - | grid | sheet(node-ref)!,cell(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.comment.delete | Delete | grid | command | grid | minus | Take the **comment** away from a cell. | - | grid | sheet(node-ref)!,cell(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.comment.get | Get | grid | command | grid | list | The **comment** of a cell: the thread and its replies. | - | grid | sheet(node-ref)!,cell(text)! | mutates=false | undoable=false"#,
    r#"grid.cell.comment.reply | Reply | grid | command | grid | pencil | **Reply** to the comment of a cell. | - | grid | sheet(node-ref)!,cell(text)!,text(text)!,author(text)? | mutates=true | undoable=true"#,
    r#"grid.cell.comment.resolve | Resolve | grid | command | grid | check | **Resolve** the comment of a cell, or leave it unresolved again. | - | grid | sheet(node-ref)!,cell(text)!,resolved(bool)? | mutates=true | undoable=true"#,
    r#"grid.cell.comment.set | Set | grid | command | grid | pencil | Put a **comment** on a cell. | - | grid | sheet(node-ref)!,cell(text)!,text(text)!,author(text)? | mutates=true | undoable=true"#,
    r#"grid.cell.date | Date | grid | command | grid | plus | Put a **date** in a cell. | - | grid | sheet(node-ref)!,cell(text)!,date(text)!,format(text)? | mutates=true | undoable=true"#,
    r#"grid.cell.fill | Fill | grid | command | grid | grid | **Fill** a cell with a colour. | - | grid | sheet(node-ref)!,cell(text)!,colour(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.font | Font | grid | command | grid | grid | **Set** the face, the size, the underline or the place of the text of a cell. | - | grid | sheet(node-ref)!,cell(text)!,name(text)?,size(number)?,underline(text)?,vert(text)? | mutates=true | undoable=true"#,
    r#"grid.cell.font.bold | Bold | grid | command | grid | grid | **Bold** the text of a cell, or take the bold away. | - | grid | sheet(node-ref)!,cell(text)!,on(bool)? | mutates=true | undoable=true"#,
    r#"grid.cell.font.colour | Colour | grid | command | grid | grid | **Colour** the text of a cell. | - | grid | sheet(node-ref)!,cell(text)!,colour(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.font.italic | Italic | grid | command | grid | grid | **Italic** the text of a cell, or take the italic away. | - | grid | sheet(node-ref)!,cell(text)!,on(bool)? | mutates=true | undoable=true"#,
    r#"grid.cell.font.strike | Strike | grid | command | grid | grid | **Strike** the text of a cell through, or take the strike away. | - | grid | sheet(node-ref)!,cell(text)!,on(bool)? | mutates=true | undoable=true"#,
    r#"grid.cell.format | Format | grid | command | grid | pencil | Put a **number format** on a cell. | - | grid | sheet(node-ref)!,cell(text)!,format(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.format.decimals | Decimals | grid | command | grid | pencil | **Move** the decimals of the format of a cell. | - | grid | sheet(node-ref)!,cell(text)!,delta(number)! | mutates=true | undoable=true"#,
    r#"grid.cell.get | Get | grid | command | grid | list | **Read** a cell whole. | - | grid | sheet(node-ref)!,cell(text)! | mutates=false | undoable=false"#,
    r#"grid.cell.link.delete | Delete | grid | command | grid | minus | Take the **link** away from a cell. | - | grid | sheet(node-ref)!,cell(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.link.get | Get | grid | command | grid | list | The **link** on a cell. | - | grid | sheet(node-ref)!,cell(text)! | mutates=false | undoable=false"#,
    r#"grid.cell.link.set | Set | grid | command | grid | pencil | Put a **link** on a cell. | - | grid | sheet(node-ref)!,cell(text)!,target(text)!,tooltip(text)? | mutates=true | undoable=true"#,
    r#"grid.cell.lock | Lock | grid | command | grid | lock | **Lock** a cell, or let it be changed. | - | grid | sheet(node-ref)!,cell(text)!,locked(bool)? | mutates=true | undoable=true"#,
    r#"grid.cell.note.delete | Delete | grid | command | grid | minus | Take the **note** away from a cell. | - | grid | sheet(node-ref)!,cell(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.note.get | Get | grid | command | grid | list | The **note** on a cell. | - | grid | sheet(node-ref)!,cell(text)! | mutates=false | undoable=false"#,
    r#"grid.cell.note.set | Set | grid | command | grid | pencil | Put a **note** on a cell. | - | grid | sheet(node-ref)!,cell(text)!,text(text)!,author(text)? | mutates=true | undoable=true"#,
    r#"grid.cell.set | Set | grid | command | grid | plus | Put a **value** in a cell. | - | grid | sheet(node-ref)!,cell(text)!,value(json)! | mutates=true | undoable=true"#,
    r#"grid.cell.style | Style | grid | command | grid | paint | A **cell** names a style. | - | grid | sheet(node-ref)!,cell(text)!,style(text)! | mutates=true | undoable=true"#,
    r#"grid.cell.text | Text | grid | command | grid | list | The **text** a cell shows. | - | grid | sheet(node-ref)!,cell(text)! | mutates=false | undoable=false"#,
    r#"grid.cell.time | Time | grid | command | grid | plus | Put a **time** in a cell. | - | grid | sheet(node-ref)!,cell(text)!,time(text)!,format(text)? | mutates=true | undoable=true"#,
    r#"grid.chart.add | Add | grid | command | grid | chart | **Add** a chart to a sheet. | - | grid | sheet(node-ref)!,range(text)!,kind(text)?,title(text)?,legend(text)?,data_labels(bool)?,gridlines(bool)?,x_title(text)?,y_title(text)? | mutates=true | undoable=true"#,
    r#"grid.chart.element | Element | grid | command | grid | chart | **Turn** an **element** of a chart on or off. | - | grid | chart(node-ref)!,element(text)!,on(bool)?,position(text)? | mutates=true | undoable=true"#,
    r#"grid.chart.read | Read | grid | command | grid | chart | **Read** a chart: what it reads and what it shows. | - | grid | chart(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.chart.remove | Remove | grid | command | grid | chart | **Take** a chart away. | - | grid | chart(node-ref)! | mutates=true | undoable=true"#,
    r#"grid.chart.set | Set | grid | command | grid | chart | **Change** a chart: its range, its kind or its title. | - | grid | chart(node-ref)!,range(text)?,kind(text)?,title(text)?,legend(text)?,data_labels(bool)?,gridlines(bool)?,x_title(text)?,y_title(text)? | mutates=true | undoable=true"#,
    r#"grid.chart.trendline | Trendline | grid | command | grid | chart | **The trendline of a chart**: the fit of a kind of fit to the values of its series. | - | grid | chart(node-ref)!,kind(text)!,order(number)?,period(number)?,series(text)? | mutates=false | undoable=false"#,
    r#"grid.column.autofit | Autofit | grid | command | grid | columns | **Make** a column **as wide as its text**. | - | grid | sheet(node-ref)!,column(text)! | mutates=true | undoable=true"#,
    r#"grid.column.delete | Delete | grid | command | grid | minus | **Delete** a column from a sheet. | - | grid | sheet(node-ref)!,column(text)!,count(number)? | mutates=true | undoable=true"#,
    r#"grid.column.detail.hide | Hide | grid | command | grid | eye | **Hide** the **detail** of a group of columns. | - | grid | sheet(node-ref)!,columns(text)!,level(number)? | mutates=true | undoable=true"#,
    r#"grid.column.detail.show | Show | grid | command | grid | eye | **Show** the **detail** of a group of columns. | - | grid | sheet(node-ref)!,columns(text)!,level(number)? | mutates=true | undoable=true"#,
    r#"grid.column.group | Group | grid | command | grid | grid | **Group** a range of columns. | - | grid | sheet(node-ref)!,columns(text)! | mutates=true | undoable=true"#,
    r#"grid.column.groups | Groups | grid | command | grid | list | **Read** the groups of the columns of a sheet. | - | grid | sheet(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.column.hide | Hide | grid | command | grid | eye | **Hide** a column. | - | grid | sheet(node-ref)!,column(text)! | mutates=true | undoable=true"#,
    r#"grid.column.insert | Insert | grid | command | grid | plus | **Insert** a column into a sheet. | - | grid | sheet(node-ref)!,column(text)!,count(number)? | mutates=true | undoable=true"#,
    r#"grid.column.show | Show | grid | command | grid | eye | **Show** a column that was hidden. | - | grid | sheet(node-ref)!,column(text)! | mutates=true | undoable=true"#,
    r#"grid.column.ungroup | Ungroup | grid | command | grid | grid | **Ungroup** a range of columns. | - | grid | sheet(node-ref)!,columns(text)! | mutates=true | undoable=true"#,
    r#"grid.column.width | Width | grid | command | grid | columns | The **width** of a column, in characters. | - | grid | sheet(node-ref)!,column(text)!,width(number)! | mutates=true | undoable=true"#,
    r#"grid.conditional.check | Check | grid | command | grid | list | The **cells** of a range that a conditional format holds for. | - | grid | sheet(node-ref)!,range(text)! | mutates=false | undoable=false"#,
    r#"grid.conditional.clear | Clear | grid | command | grid | trash | Take a **conditional format** away. | - | grid | sheet(node-ref)!,range(text)! | mutates=true | undoable=true"#,
    r#"grid.conditional.set | Set | grid | command | grid | brush | Put a **conditional format** on a range of cells. | - | grid | sheet(node-ref)!,range(text)!,op(text)!,value(text)!,format(text)?,fill(text)?,stop(bool)?,priority(number)? | mutates=true | undoable=true"#,
    r#"grid.data.consolidate | Consolidate | grid | command | grid | list | **Consolidate** the ranges of cells into one table. | - | grid | sources(json)!,sheet(node-ref)!,destination(text)!,function(text)?,top_row(bool)?,left_column(bool)? | mutates=true | undoable=true"#,
    r#"grid.data.subtotal | Subtotal | grid | command | grid | list | The **subtotal** of the groups of rows. | - | grid | sheet(node-ref)!,range(text)!,group_by(text)!,function(text)?,columns(json)?,header(bool)? | mutates=true | undoable=true"#,
    r#"grid.data.table | Table | grid | command | grid | list | The **data table** of a sheet. | - | grid | sheet(node-ref)!,range(text)!,row_input(text)?,column_input(text)? | mutates=true | undoable=true"#,
    r#"grid.dependents | Dependents | grid | command | grid | list | The **dependents** of a cell: the cells whose formulas read it. | - | grid | sheet(node-ref)!,cell(text)!,all(bool)? | mutates=false | undoable=false"#,
    r#"grid.duplicates.remove | Remove | grid | command | grid | minus | **Remove** the **duplicates** of a range. | - | grid | sheet(node-ref)!,range(text)?,columns(text)?,header(bool)? | mutates=true | undoable=true"#,
    r#"grid.errors | Errors | grid | command | grid | list | **Read** the **errors** of a sheet. | - | grid | sheet(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.export.csv | Csv | grid | command | grid | list | **Export** a range of a sheet as a text of values. | - | grid | sheet(node-ref)!,range(text)?,delimiter(text)? | mutates=false | undoable=false"#,
    r#"grid.filter.advanced | Advanced | grid | command | grid | filter | **Filter** a list of rows by a **criteria range** — the tree's own advanced filter. | - | grid | sheet(node-ref)!,range(text)!,criteria(text)!,copy_to(text)?,unique(bool)? | mutates=true | undoable=true"#,
    r#"grid.filter.clear | Clear | grid | command | grid | filter | Take the **filter** away from a sheet. | - | grid | sheet(node-ref)! | mutates=true | undoable=true"#,
    r#"grid.filter.groups.set | Set | grid | command | grid | filter | **Filter** a sheet by **groups** of criteria. | - | grid | sheet(node-ref)!,groups(json)! | mutates=true | undoable=true"#,
    r#"grid.filter.rows | Rows | grid | command | grid | list | The **rows** a sheet's filter hides. | - | grid | sheet(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.filter.set | Set | grid | command | grid | filter | **Filter** a sheet by a column. | - | grid | sheet(node-ref)!,column(text)!,op(text)!,value(text)! | mutates=true | undoable=true"#,
    r#"grid.filter.values | Values | grid | command | grid | filter | **Filter** a sheet by a **list of values**. | - | grid | sheet(node-ref)!,column(text)!,values(json)!,blanks(bool)? | mutates=true | undoable=true"#,
    r#"grid.find | Find | grid | command | grid | list | **Find** the cells whose text holds what was looked for. | - | grid | sheet(node-ref)?,what(text)!,match_case(bool)?,whole_cell(bool)?,formulas(bool)? | mutates=false | undoable=false"#,
    r#"grid.find.special | Special | grid | command | grid | list | The **special** cells of a sheet. | - | grid | sheet(node-ref)!,kind(text)!,range(text)?,cell(text)? | mutates=false | undoable=false"#,
    r#"grid.flash.fill | Fill | grid | command | grid | plus | **Flash fill** a column: learn from the rows that are filled and fill the rows that are not. | - | grid | sheet(node-ref)!,at(text)! | mutates=true | undoable=true"#,
    r#"grid.formula.set | Set | grid | command | grid | plus | Put a **formula** in a cell. | - | grid | sheet(node-ref)!,cell(text)!,formula(text)! | mutates=true | undoable=true"#,
    r#"grid.formula.values | Values | grid | command | grid | list | Turn the formulas of a range into the **values** they give. | - | grid | sheet(node-ref)!,range(text)! | mutates=true | undoable=true"#,
    r#"grid.goal | Goal | grid | command | grid | list | The **goal**: the input that gives a result. | - | grid | sheet(node-ref)!,input(text)!,result(text)!,target(number)!,from(number)!,to(number)! | mutates=false | undoable=false"#,
    r#"grid.goal.scan | Scan | grid | command | grid | list | The **crossings** of a sheet: every place the result goes past the target. | - | grid | sheet(node-ref)!,input(text)!,result(text)!,target(number)!,from(number)!,to(number)!,step(number)? | mutates=false | undoable=false"#,
    r#"grid.import.csv | Csv | grid | command | grid | plus | **Import** a text of values into a workbook, as a sheet. | - | grid | workbook(node-ref)!,text(text)!,name(text)?,delimiter(text)? | mutates=true | undoable=true"#,
    r#"grid.import.json | Json | grid | command | grid | plus | **Import** a list of objects into a workbook, as a sheet. | - | grid | workbook(node-ref)!,data(json)!,name(text)? | mutates=true | undoable=true"#,
    r#"grid.merge.across | Across | grid | command | grid | grid | **Merge** a range of cells **across** its rows. | - | grid | sheet(node-ref)!,range(text)! | mutates=true | undoable=true"#,
    r#"grid.merge.center | Center | grid | command | grid | grid | **Merge** a range of cells and **centre** its text. | - | grid | sheet(node-ref)!,range(text)! | mutates=true | undoable=true"#,
    r#"grid.merge.clear | Clear | grid | command | grid | grid | **Unmerge** a range of cells. | - | grid | sheet(node-ref)!,range(text)! | mutates=true | undoable=true"#,
    r#"grid.merge.set | Set | grid | command | grid | grid | **Merge** a range of cells. | - | grid | sheet(node-ref)!,range(text)! | mutates=true | undoable=true"#,
    r#"grid.name.delete | Delete | grid | command | grid | trash | Take a **name** away from a workbook. | - | grid | workbook(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"grid.name.get | Get | grid | command | grid | list | Read a **name** of a workbook. | - | grid | workbook(node-ref)!,name(text)! | mutates=false | undoable=false"#,
    r#"grid.name.set | Set | grid | command | grid | tag | Write a **name** into a workbook. | - | grid | workbook(node-ref)!,name(text)!,refers_to(text)! | mutates=true | undoable=true"#,
    r#"grid.numbers.convert | Convert | grid | command | grid | pencil | **Turn** the numbers stored as text of a sheet into numbers. | - | grid | sheet(node-ref)! | mutates=true | undoable=true"#,
    r#"grid.numbers.stored.as.text | Text | grid | command | grid | list | **Read** the cells of a sheet that hold a number as text. | - | grid | sheet(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.outline.levels | Levels | grid | command | grid | grid | **Show** the rows or the columns down to an outline level. | - | grid | sheet(node-ref)!,rows(number)?,cols(number)? | mutates=true | undoable=true"#,
    r#"grid.picture.read | Read | grid | command | grid | image | **Read** the picture of a cell: where its bytes are and how big it is. | - | grid | sheet(node-ref)!,cell(text)? | mutates=false | undoable=false"#,
    r#"grid.picture.remove | Remove | grid | command | grid | trash | **Take** the picture of a cell away. | - | grid | sheet(node-ref)!,cell(text)! | mutates=true | undoable=true"#,
    r#"grid.picture.set | Set | grid | command | grid | image | **Set** the picture of a cell. | - | grid | sheet(node-ref)!,cell(text)!,path(text)!,alt(text)? | mutates=true | undoable=true"#,
    r#"grid.pivot.create | Create | grid | command | grid | grid | **Make** a pivot of a sheet. | - | grid | sheet(node-ref)!,name(text)!,range(text)!,row(text)!,column(text)?,value(text)?,aggregate(text)? | mutates=true | undoable=true"#,
    r#"grid.pivot.detail | Detail | grid | command | grid | grid | **Read** the rows that stand behind a **group** of a pivot. | - | grid | sheet(node-ref)!,name(text)!,group(text)! | mutates=false | undoable=false"#,
    r#"grid.pivot.fields | Fields | grid | command | grid | list | **Read** the **fields** of a pivot. | - | grid | sheet(node-ref)!,name(text)! | mutates=false | undoable=false"#,
    r#"grid.pivot.filter | Filter | grid | command | grid | grid | **Say** which **items** of a field a pivot keeps. | - | grid | sheet(node-ref)!,name(text)!,field(text)!,selected(json)? | mutates=true | undoable=true"#,
    r#"grid.pivot.grandtotals | Grandtotals | grid | command | grid | grid | **Say** where the **grand totals** of a pivot are. | - | grid | sheet(node-ref)!,name(text)!,mode(text)! | mutates=true | undoable=true"#,
    r#"grid.pivot.group | Group | grid | command | grid | grid | **Say** how the items of a **date field** are grouped. | - | grid | sheet(node-ref)!,name(text)!,field(text)!,by(text)! | mutates=true | undoable=true"#,
    r#"grid.pivot.read | Read | grid | command | grid | grid | **Read** the summary of a pivot. | - | grid | sheet(node-ref)!,name(text)! | mutates=false | undoable=false"#,
    r#"grid.pivot.remove | Remove | grid | command | grid | grid | **Take away** a pivot of a sheet. | - | grid | sheet(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"grid.pivot.showas | Showas | grid | command | grid | grid | **Say** how the values of a pivot are **shown**. | - | grid | sheet(node-ref)!,name(text)!,show_as(text)! | mutates=true | undoable=true"#,
    r#"grid.pivot.sort | Sort | grid | command | grid | grid | **Say** in what **order** the items of a field are. | - | grid | sheet(node-ref)!,name(text)!,field(text)!,order(text)? | mutates=true | undoable=true"#,
    r#"grid.pivot.subtotals | Subtotals | grid | command | grid | grid | **Say** where the **subtotals** of a pivot are. | - | grid | sheet(node-ref)!,name(text)!,mode(text)! | mutates=true | undoable=true"#,
    r#"grid.precedents | Precedents | grid | command | grid | list | The **precedents** of a cell: the areas its formula reads. | - | grid | sheet(node-ref)!,cell(text)!,all(bool)? | mutates=false | undoable=false"#,
    r#"grid.print.area | Area | grid | command | grid | page | **Set** the area a sheet prints. | - | grid | sheet(node-ref)!,range(text)? | mutates=true | undoable=true"#,
    r#"grid.print.break | Break | grid | command | grid | page | **Put** a break in a sheet, or take it away. | - | grid | sheet(node-ref)!,row(number)?,column(text)?,clear(bool)? | mutates=true | undoable=true"#,
    r#"grid.print.fit | Fit | grid | command | grid | page | **Print** a sheet to fit: so many pages wide and so many pages tall. | - | grid | sheet(node-ref)!,wide(number)!,tall(number)! | mutates=true | undoable=true"#,
    r#"grid.print.footer | Footer | grid | command | grid | page | **Set** the footer of a sheet — or take it away. | - | grid | sheet(node-ref)!,page(text)?,text(text)! | mutates=true | undoable=true"#,
    r#"grid.print.gridlines | Gridlines | grid | command | grid | page | **Print** the gridlines of a sheet, or do not. | - | grid | sheet(node-ref)!,on(bool)! | mutates=true | undoable=true"#,
    r#"grid.print.header | Header | grid | command | grid | page | **Set** the header of a sheet — or take it away. | - | grid | sheet(node-ref)!,page(text)?,text(text)! | mutates=true | undoable=true"#,
    r#"grid.print.headings | Headings | grid | command | grid | page | **Print** the headings of a sheet, or do not. | - | grid | sheet(node-ref)!,on(bool)! | mutates=true | undoable=true"#,
    r#"grid.print.margins | Margins | grid | command | grid | page | **Set** the margins of a sheet. | - | grid | sheet(node-ref)!,preset(text)?,left(number)?,right(number)?,top(number)?,bottom(number)? | mutates=true | undoable=true"#,
    r#"grid.print.orientation | Orientation | grid | command | grid | page | **Set** the orientation of a sheet. | - | grid | sheet(node-ref)!,orientation(text)! | mutates=true | undoable=true"#,
    r#"grid.print.pages | Pages | grid | command | grid | list | **Read** the pages of a sheet. | - | grid | sheet(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.print.paper | Paper | grid | command | grid | page | **Set** the paper of a sheet. | - | grid | sheet(node-ref)!,paper(text)! | mutates=true | undoable=true"#,
    r#"grid.print.read | Read | grid | command | grid | list | **Read** the setup of a sheet. | - | grid | sheet(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.print.scale | Scale | grid | command | grid | page | **Set** the scale a sheet prints at. | - | grid | sheet(node-ref)!,percent(number)! | mutates=true | undoable=true"#,
    r#"grid.print.titles | Titles | grid | command | grid | page | **Set** the titles of a sheet: the rows and the columns that are printed on **every** page. | - | grid | sheet(node-ref)!,rows(text)?,columns(text)? | mutates=true | undoable=true"#,
    r#"grid.range.fill | Fill | grid | command | grid | plus | **Fill** a range from a cell or a range of cells. | - | grid | sheet(node-ref)!,from(text)!,to(text)!,mode(text)? | mutates=true | undoable=true"#,
    r#"grid.range.read | Read | grid | command | grid | list | **Read** a range of cells. | - | grid | sheet(node-ref)!,range(text)?,formulas(bool)? | mutates=false | undoable=false"#,
    r#"grid.replace | Replace | grid | command | grid | pencil | **Replace** text in the cells of a sheet. | - | grid | sheet(node-ref)?,what(text)!,with(text)!,match_case(bool)?,whole_cell(bool)?,formulas(bool)? | mutates=true | undoable=true"#,
    r#"grid.row.autofit | Autofit | grid | command | grid | rows | **Make** a row **as tall as its text**. | - | grid | sheet(node-ref)!,row(number)! | mutates=true | undoable=true"#,
    r#"grid.row.delete | Delete | grid | command | grid | minus | **Delete** a row from a sheet. | - | grid | sheet(node-ref)!,row(number)!,count(number)? | mutates=true | undoable=true"#,
    r#"grid.row.detail.hide | Hide | grid | command | grid | eye | **Hide** the **detail** of a group of rows. | - | grid | sheet(node-ref)!,rows(text)!,level(number)? | mutates=true | undoable=true"#,
    r#"grid.row.detail.show | Show | grid | command | grid | eye | **Show** the **detail** of a group of rows. | - | grid | sheet(node-ref)!,rows(text)!,level(number)? | mutates=true | undoable=true"#,
    r#"grid.row.group | Group | grid | command | grid | grid | **Group** a range of rows. | - | grid | sheet(node-ref)!,rows(text)! | mutates=true | undoable=true"#,
    r#"grid.row.groups | Groups | grid | command | grid | list | **Read** the groups of a sheet. | - | grid | sheet(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.row.height | Height | grid | command | grid | rows | The **height** of a row, in points. | - | grid | sheet(node-ref)!,row(number)!,height(number)! | mutates=true | undoable=true"#,
    r#"grid.row.hide | Hide | grid | command | grid | eye | **Hide** a row. | - | grid | sheet(node-ref)!,row(number)! | mutates=true | undoable=true"#,
    r#"grid.row.insert | Insert | grid | command | grid | plus | **Insert** a row into a sheet. | - | grid | sheet(node-ref)!,row(number)!,count(number)? | mutates=true | undoable=true"#,
    r#"grid.row.show | Show | grid | command | grid | eye | **Show** a row that was hidden. | - | grid | sheet(node-ref)!,row(number)! | mutates=true | undoable=true"#,
    r#"grid.row.ungroup | Ungroup | grid | command | grid | grid | **Ungroup** a range of rows. | - | grid | sheet(node-ref)!,rows(text)! | mutates=true | undoable=true"#,
    r#"grid.scenario.add | Add | grid | command | grid | plus | **Add** a **scenario** to a workbook. | - | grid | workbook(node-ref)!,sheet(node-ref)!,name(text)!,changing(json)!,values(json)?,comment(text)?,replace(bool)? | mutates=true | undoable=true"#,
    r#"grid.scenario.delete | Delete | grid | command | grid | trash | **Delete** a **scenario** of a workbook. | - | grid | workbook(node-ref)!,name(text)!,sheet(node-ref)? | mutates=true | undoable=true"#,
    r#"grid.scenario.list | List | grid | command | grid | list | **List** the **scenarios** of a workbook. | - | grid | workbook(node-ref)!,sheet(node-ref)? | mutates=false | undoable=false"#,
    r#"grid.scenario.show | Show | grid | command | grid | play | **Show** a **scenario**: the changing cells hold the values of it. | - | grid | workbook(node-ref)!,name(text)!,sheet(node-ref)? | mutates=true | undoable=true"#,
    r#"grid.scenario.summary | Summary | grid | command | grid | list | The **summary** of the scenarios of a sheet: a sheet of its own that says what every scenario does to the book. | - | grid | workbook(node-ref)!,sheet(node-ref)!,result_cells(text)? | mutates=true | undoable=true"#,
    r#"grid.sheet.add | Add | grid | command | grid | plus | Put a **sheet** in a workbook. | - | grid | workbook(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"grid.sheet.color | Color | grid | command | grid | paint | The **colour** of a sheet's tab. | - | grid | sheet(node-ref)!,color(text)! | mutates=true | undoable=true"#,
    r#"grid.sheet.edit.read | Read | grid | command | grid | lock | **Read** the **edit ranges** of a sheet: the ranges that stay editable while the sheet is protected. | - | grid | sheet(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.sheet.edit.set | Set | grid | command | grid | lock | **Set** the **edit ranges** of a sheet: the ranges that stay editable while the sheet is protected. | - | grid | sheet(node-ref)!,ranges(json)! | mutates=true | undoable=true"#,
    r#"grid.sheet.freeze | Freeze | grid | command | grid | lock | **Freeze** the panes of a sheet at a cell. | - | grid | sheet(node-ref)!,cell(text)! | mutates=true | undoable=true"#,
    r#"grid.sheet.freeze.delete | Delete | grid | command | grid | minus | **Unfreeze** the panes of a sheet. | - | grid | sheet(node-ref)! | mutates=true | undoable=true"#,
    r#"grid.sheet.get | Get | grid | command | grid | list | Read a **sheet**. | - | grid | sheet(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.sheet.hide | Hide | grid | command | grid | eye | **Hide** a sheet. | - | grid | sheet(node-ref)! | mutates=true | undoable=true"#,
    r#"grid.sheet.move | Move | grid | command | grid | arrow | **Move** a sheet in the order of the tabs. | - | grid | sheet(node-ref)!,to(number)! | mutates=true | undoable=true"#,
    r#"grid.sheet.protect | Protect | grid | command | grid | lock | **Protect** a sheet. | - | grid | sheet(node-ref)!,sort(bool)?,filter(bool)?,format(bool)? | mutates=true | undoable=true"#,
    r#"grid.sheet.remove | Remove | grid | command | grid | minus | **Take** a sheet away from its workbook. | - | grid | sheet(node-ref)! | mutates=true | undoable=true"#,
    r#"grid.sheet.rename | Rename | grid | command | grid | pencil | **Rename** a sheet. | - | grid | sheet(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"grid.sheet.summary | Summary | grid | command | grid | grid | **Say** where the summary rows of a sheet sit. | - | grid | sheet(node-ref)!,below(bool)?,right(bool)? | mutates=true | undoable=true"#,
    r#"grid.sheet.unhide | Unhide | grid | command | grid | eye | **Unhide** a sheet. | - | grid | sheet(node-ref)! | mutates=true | undoable=true"#,
    r#"grid.sheet.unprotect | Unprotect | grid | command | grid | unlock | **Unprotect** a sheet: the protection is gone. | - | grid | sheet(node-ref)! | mutates=true | undoable=true"#,
    r#"grid.sheet.view | View | grid | command | grid | eye | Put the **view** of a sheet. | - | grid | sheet(node-ref)!,gridlines(bool)?,headings(bool)?,zeros(bool)?,right_to_left(bool)?,outline_symbols(bool)?,show_formulas(bool)? | mutates=true | undoable=true"#,
    r#"grid.sheet.zoom | Zoom | grid | command | grid | zoom | The **zoom** of a sheet, as a percentage. | - | grid | sheet(node-ref)!,percent(number)! | mutates=true | undoable=true"#,
    r#"grid.slicer.add | Add | grid | command | grid | filter | **Add** a slicer to a table of a sheet. | - | grid | sheet(node-ref)!,table(text)!,column(text)? | mutates=true | undoable=true"#,
    r#"grid.slicer.clear | Clear | grid | command | grid | filter | **Clear** a slicer: every value is kept, and the filter is taken away. | - | grid | sheet(node-ref)!,slicer(number)? | mutates=true | undoable=true"#,
    r#"grid.slicer.read | Read | grid | command | grid | list | **Read** a slicer: its values, and which of them are kept. | - | grid | sheet(node-ref)!,slicer(number)? | mutates=false | undoable=false"#,
    r#"grid.slicer.remove | Remove | grid | command | grid | filter | **Remove** a slicer, and the filter it wrote. | - | grid | sheet(node-ref)!,slicer(number)? | mutates=true | undoable=true"#,
    r#"grid.slicer.toggle | Toggle | grid | command | grid | filter | **Toggle** a value of a slicer. | - | grid | sheet(node-ref)!,slicer(number)?,value(text)!,multi(bool)? | mutates=true | undoable=true"#,
    r#"grid.solver.solve | Solve | grid | command | grid | check | Solve a **linear program**. | - | grid | objective(json)?,constraints(json)?,bounds(json)?,maximise(bool)?,budget(number)?,integer(json)?,binary(json)?,max_nodes(number)?,gap(number)? | mutates=false | undoable=false"#,
    r#"grid.sort | Sort | grid | command | grid | arrow | **Sort** a range of cells by one of its columns. | - | grid | sheet(node-ref)!,range(text)?,column(text)?,descending(bool)?,header(bool)? | mutates=true | undoable=true"#,
    r#"grid.sparkline.read | Read | grid | command | grid | chart | **Read** the sparkline of a cell: what it reads and what it shows. | - | grid | sheet(node-ref)!,cell(text)? | mutates=false | undoable=false"#,
    r#"grid.sparkline.remove | Remove | grid | command | grid | trash | **Take** the sparkline of a cell away. | - | grid | sheet(node-ref)!,cell(text)! | mutates=true | undoable=true"#,
    r#"grid.sparkline.set | Set | grid | command | grid | chart | **Set** the sparkline of a cell. | - | grid | sheet(node-ref)!,cell(text)!,range(text)!,kind(text)?,colour(text)?,markers(bool)? | mutates=true | undoable=true"#,
    r#"grid.style.add | Add | grid | command | grid | plus | A **style** is added to a workbook. | - | grid | workbook(node-ref)!,name(text)!,format(text)! | mutates=true | undoable=true"#,
    r#"grid.style.align | Align | grid | command | grid | grid | The **alignment** of a style is changed. | - | grid | workbook(node-ref)!,name(text)!,horizontal(text)?,vertical(text)?,indent(number)?,wrap(bool)? | mutates=true | undoable=true"#,
    r#"grid.style.font | Font | grid | command | grid | grid | The **font** of a style is changed. | - | grid | workbook(node-ref)!,name(text)!,face(text)?,size(number)?,underline(text)?,vert(text)? | mutates=true | undoable=true"#,
    r#"grid.style.lock | Lock | grid | command | grid | lock | The **lock** of a style is changed. | - | grid | workbook(node-ref)!,name(text)!,locked(bool)! | mutates=true | undoable=true"#,
    r#"grid.style.remove | Remove | grid | command | grid | minus | A **style** is taken away. | - | grid | workbook(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"grid.style.set | Set | grid | command | grid | pencil | The **format** of a style is changed. | - | grid | workbook(node-ref)!,name(text)!,format(text)! | mutates=true | undoable=true"#,
    r#"grid.table.create | Create | grid | command | grid | grid | **Make** a table of a sheet. | - | grid | sheet(node-ref)!,name(text)!,range(text)! | mutates=true | undoable=true"#,
    r#"grid.table.fields | Fields | grid | command | grid | grid | **Read** the fields of a table. | - | grid | sheet(node-ref)!,name(text)! | mutates=false | undoable=false"#,
    r#"grid.table.label | Label | grid | command | grid | grid | **Say** the **label** of the **total row** of a table. | - | grid | sheet(node-ref)!,name(text)!,label(text)? | mutates=true | undoable=true"#,
    r#"grid.table.read | Read | grid | command | grid | grid | **Read** the tables of a sheet. | - | grid | sheet(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.table.remove | Remove | grid | command | grid | grid | **Remove** a table of a sheet. | - | grid | sheet(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"grid.table.rename | Rename | grid | command | grid | grid | **Rename** a table of a sheet. | - | grid | sheet(node-ref)!,name(text)!,to(text)! | mutates=true | undoable=true"#,
    r#"grid.table.resize | Resize | grid | command | grid | grid | **Resize** a table of a sheet. | - | grid | sheet(node-ref)!,name(text)!,range(text)! | mutates=true | undoable=true"#,
    r#"grid.table.totals | Totals | grid | command | grid | grid | **Say** whether a table has a **total row**. | - | grid | sheet(node-ref)!,name(text)!,totals(bool)?,way(text)? | mutates=true | undoable=true"#,
    r#"grid.table.way | Way | grid | command | grid | grid | **Say** the way a **column** of a table is totalled in. | - | grid | sheet(node-ref)!,name(text)!,column(text)!,way(text)! | mutates=true | undoable=true"#,
    r#"grid.text.columns | Columns | grid | command | grid | scissors | **Cut** the text of a column into columns. | - | grid | sheet(node-ref)!,column(text)!,delimiter(text)?,qualifier(text)?,treat_consecutive(bool)?,destination(text)? | mutates=true | undoable=true"#,
    r#"grid.validation.check | Check | grid | command | grid | check | Check a **value** against the validation rules of a cell. | - | grid | sheet(node-ref)!,cell(text)!,input(text)! | mutates=false | undoable=false"#,
    r#"grid.validation.clear | Clear | grid | command | grid | minus | Take the **validation rules** off a range of a sheet. | - | grid | sheet(node-ref)!,range(text)! | mutates=true | undoable=true"#,
    r#"grid.validation.set | Set | grid | command | grid | check | Put a **validation rule** on a range of a sheet. | - | grid | sheet(node-ref)!,range(text)!,kind(text)!,operator(text)?,formula1(text)?,formula2(text)?,ignore_blank(bool)?,dropdown(bool)?,input_title(text)?,input_message(text)?,error_title(text)?,error_message(text)?,error_style(text)? | mutates=true | undoable=true"#,
    r#"grid.whatif | Whatif | grid | command | grid | list | Ask a sheet what it would say for a list of values. | - | grid | sheet(node-ref)!,input(text)!,result(text)!,values(json)! | mutates=false | undoable=false"#,
    r#"grid.workbook.add | Add | grid | command | grid | plus | Put a **workbook** in the document. | - | grid | name(text)! | mutates=true | undoable=true"#,
    r#"grid.workbook.date.system | System | grid | command | grid | pencil | **Set** the **date system** of a workbook. | - | grid | workbook(node-ref)!,system(text)! | mutates=true | undoable=true"#,
    r#"grid.workbook.get | Get | grid | command | grid | list | Read a **workbook**. | - | grid | workbook(node-ref)! | mutates=false | undoable=false"#,
    r#"grid.workbook.list.add | Add | grid | command | grid | plus | **Add** a **custom list** to a workbook. | - | grid | workbook(node-ref)!,words(text)! | mutates=true | undoable=true"#,
    r#"grid.workbook.list.remove | Remove | grid | command | grid | minus | **Take away** a **custom list** of a workbook. | - | grid | workbook(node-ref)!,list(number)! | mutates=true | undoable=true"#,
];

/// The commands of the grid battery, in the order of their ids.
fn the_battery() -> Vec<CommandSpec> {
    let ecosystem = craftkit_studio::ecosystem().expect("the ecosystem is not a mess");
    let mut commands: Vec<CommandSpec> =
        ecosystem.commands.all().iter().filter(|command| command.area == "grid").cloned().collect();
    commands.sort_by(|left, right| left.id.cmp(right.id));
    commands
}

/// One command, written down.
fn written(command: &CommandSpec) -> String {
    let params: Vec<String> = command
        .params
        .iter()
        .map(|param| {
            format!("{}({}){}", param.name, param.kind.json_name(), if param.required { "!" } else { "?" })
        })
        .collect();
    format!(
        "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | mutates={} | undoable={}",
        command.id,
        command.label,
        command.area,
        command.capability,
        command.layer,
        command.icon,
        command.doc,
        command.shortcut.unwrap_or("-"),
        command.menu.join("/"),
        params.join(","),
        command.mutates,
        command.undoable
    )
}

/// The registry says what the snapshot says.
#[test]
fn the_registry_is_the_snapshot() {
    let commands = the_battery();
    assert!(!commands.is_empty(), "the battery has no commands");
    let found: Vec<String> = commands.iter().map(written).collect();
    if std::env::var("CRAFTKIT_DUMP").is_ok() {
        for line in &found {
            println!("    r#\"{line}\"#,");
        }
        return;
    }
    assert_eq!(found, EXPECTED, "the registry is not what it was");
}
