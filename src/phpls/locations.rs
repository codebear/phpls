use std::convert::TryInto;

use phpanalyzer::symboldata::FileLocation;
use rust_lsp::lsp_types::{Location, Position, Range};
use url::Url;

///
///
pub fn file_location_to_location(file_location: FileLocation) -> Location {
    let uri: Url = Url::parse(&file_location.uri.to_string_lossy().to_string())
        .unwrap_or_else(|_| Url::parse("file://unknown_or_unparseable").unwrap());
    let range = Range {
        start: Position {
            line: file_location.start.line.try_into().unwrap(),
            character: file_location.start.column.try_into().unwrap(),
            // void
        },
        end: Position {
            line: file_location.end.line.try_into().unwrap(),
            character: file_location.end.column.try_into().unwrap(),
            // void
        },
    };
    Location::new(uri, range)
}

pub fn file_locations_to_locations(file_locations: Vec<FileLocation>) -> Vec<Location> {
    let mut file_locations = file_locations;
    file_locations
        
        .drain(..)
        .map(file_location_to_location)
        .collect()
}
