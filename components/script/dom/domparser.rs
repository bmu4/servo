/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use document_loader::DocumentLoader;
use dom::bindings::codegen::Bindings::DOMParserBinding;
use dom::bindings::codegen::Bindings::DOMParserBinding::DOMParserMethods;
use dom::bindings::codegen::Bindings::DOMParserBinding::SupportedType::Application_xhtml_xml;
use dom::bindings::codegen::Bindings::DOMParserBinding::SupportedType::Application_xml;
use dom::bindings::codegen::Bindings::DOMParserBinding::SupportedType::Text_html;
use dom::bindings::codegen::Bindings::DOMParserBinding::SupportedType::Text_xml;
use dom::bindings::codegen::Bindings::DocumentBinding::DocumentReadyState;
use dom::bindings::codegen::Bindings::WindowBinding::WindowMethods;
use dom::bindings::error::Fallible;
use dom::bindings::js::{JS, Root};
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::bindings::str::DOMString;
use dom::document::{Document, IsHTMLDocument};
use dom::document::DocumentSource;
use dom::globalscope::GlobalScope;
use dom::servoparser::html::{ParseContext, parse_html};
use dom::servoparser::xml::{self, parse_xml};
use dom::window::Window;

#[dom_struct]
pub struct DOMParser {
    reflector_: Reflector,
    window: JS<Window>, // XXXjdm Document instead?
}

impl DOMParser {
    fn new_inherited(window: &Window) -> DOMParser {
        DOMParser {
            reflector_: Reflector::new(),
            window: JS::from_ref(window),
        }
    }

    pub fn new(window: &Window) -> Root<DOMParser> {
        reflect_dom_object(box DOMParser::new_inherited(window),
                           window,
                           DOMParserBinding::Wrap)
    }

    pub fn Constructor(global: &GlobalScope) -> Fallible<Root<DOMParser>> {
        Ok(DOMParser::new(global.as_window()))
    }
}

impl DOMParserMethods for DOMParser {
    // https://domparsing.spec.whatwg.org/#the-domparser-interface
    fn ParseFromString(&self,
                       s: DOMString,
                       ty: DOMParserBinding::SupportedType)
                       -> Fallible<Root<Document>> {
        let url = self.window.get_url();
        let content_type =
            DOMString::from(DOMParserBinding::SupportedTypeValues::strings[ty as usize]);
        let doc = self.window.Document();
        let loader = DocumentLoader::new(&*doc.loader());
        match ty {
            Text_html => {
                let document = Document::new(&self.window,
                                             None,
                                             Some(url.clone()),
                                             IsHTMLDocument::HTMLDocument,
                                             Some(content_type),
                                             None,
                                             DocumentSource::FromParser,
                                             loader,
                                             None,
                                             None);
                parse_html(&document, s, url, ParseContext::Owner(None));
                document.set_ready_state(DocumentReadyState::Complete);
                Ok(document)
            }
            Text_xml | Application_xml | Application_xhtml_xml => {
                // FIXME: this should probably be FromParser when we actually parse the string (#3756).
                let document = Document::new(&self.window,
                                             None,
                                             Some(url.clone()),
                                             IsHTMLDocument::NonHTMLDocument,
                                             Some(content_type),
                                             None,
                                             DocumentSource::NotFromParser,
                                             loader,
                                             None,
                                             None);
                parse_xml(&document, s, url, xml::ParseContext::Owner(None));
                Ok(document)
            }
        }
    }
}
