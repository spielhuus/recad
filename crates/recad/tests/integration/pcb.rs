#[cfg(test)]
mod tests {
    use recad::pcb::{FootprintType, LayerType};
    use recad::Pcb;
    use std::path::Path;

    #[test]
    fn test_parse_full_summe_pcb_strict() {
        let pcb_path = Path::new("tests/files/summe/summe.kicad_pcb");
        if !pcb_path.exists() {
            println!(
                "Test skipped: file {} not found. Save the snippet to run this test.",
                pcb_path.display()
            );
            return;
        }

        let pcb = Pcb::load(pcb_path).expect("Failed to load and parse PCB file");

        // 1. Root Properties Completeness
        assert_eq!(pcb.version, "20240108", "Version mismatch or missing");
        assert_eq!(pcb.generator, "pcbnew", "Generator mismatch or missing");
        assert_eq!(
            pcb.generator_version.as_deref(),
            Some("8.0"),
            "Generator version mismatch or missing"
        );
        assert_eq!(
            pcb.paper.to_string(),
            "A4",
            "Paper size mismatch or missing"
        );

        // 2. Title Block Completeness
        let tb = &pcb.title_block;
        assert_eq!(tb.title.as_deref(), Some("summe"));
        assert_eq!(tb.date.as_deref(), Some("2021-05-30"));
        assert_eq!(tb.revision.as_deref(), Some("R01"));
        assert_eq!(tb.comment.len(), 3, "Exactly 3 comments should be parsed");

        let c4 = tb
            .comment
            .iter()
            .find(|c| c.0 == 4)
            .expect("Missing comment 4");
        assert_eq!(c4.1, "License CC BY 4.0 - Attribution 4.0 International");

        // 3. Setup & Plot Params Completeness
        let setup = pcb.setup.as_ref().expect("Setup block missing");
        assert_eq!(
            setup.pad_to_mask_clearance,
            Some(0.0),
            "pad_to_mask_clearance missing"
        );
        assert!(!setup.allow_soldermask_bridges_in_footprints);

        let plot = setup.pcbplotparams.as_ref().expect("PcbPlotParams missing");
        assert_eq!(plot.layerselection.as_deref(), Some("0x00010fc_ffffffff"));
        assert_eq!(plot.svgprecision, Some(6));
        assert!(plot.usegerberattributes);
        assert!(plot.creategerberjobfile);
        assert_eq!(plot.drillshape, Some(1));
        assert_eq!(plot.outputdirectory.as_deref(), Some(""));

        // 4. Layers Completeness
        assert_eq!(pcb.layers.len(), 20, "Exactly 20 layers should be parsed");
        let layer_32 = pcb
            .layers
            .iter()
            .find(|l| l.ordinal == 32)
            .expect("Layer 32 missing");
        assert_eq!(layer_32.canonical_name, "B.Adhes");
        assert!(matches!(layer_32.layer_type, LayerType::User));
        assert_eq!(layer_32.user_name.as_deref(), Some("B.Adhesive"));

        // 5. Nets Completeness
        assert_eq!(pcb.nets.len(), 52, "Exactly 52 nets should be parsed");
        assert_eq!(pcb.nets[1].name, "GND");
        assert_eq!(pcb.nets[51].name, "Net-(J5-PadT)");

        // 6. Graphic Lines Completeness
        assert_eq!(
            pcb.gr_lines.len(),
            4,
            "Exactly 4 root graphic lines should be parsed"
        );
        let gl = &pcb.gr_lines[0];
        assert_eq!(gl.layer, "Edge.Cuts");
        assert_eq!(gl.start.x, 50.8);
        assert_eq!(gl.start.y, 50.8);
        assert_eq!(gl.stroke.width, 0.15);

        // This will fail until `uuid` parsing fallback is added to `GrLine::try_from`
        assert_eq!(
            gl.uuid.as_deref(),
            Some("00000000-0000-0000-0000-000060977f7d"),
            "UUID not parsed for gr_line"
        );

        // 7. Zones Completeness
        assert_eq!(pcb.zones.len(), 1, "Exactly 1 zone should be parsed");
        let zone = &pcb.zones[0];
        assert_eq!(zone.net, Some(1));
        assert_eq!(zone.net_name.as_deref(), Some("GND"));
        assert_eq!(zone.layer, "F.Cu");
        assert_eq!(
            zone.uuid.as_deref(),
            Some("7ec0189c-7bbc-4dd2-a181-1084e8e11ee6"),
            "Zone UUID not parsed correctly"
        );
        assert_eq!(zone.min_thickness, Some(0.254));
        assert_eq!(
            zone.polygon.0.len(),
            4,
            "Zone bounding polygon points missing"
        );
        assert_eq!(zone.filled_polygons.len(), 4, "Filled polygon missing");
        assert!(
            zone.filled_polygons[0].pts.0.len() > 200,
            "Filled polygon coordinates incomplete"
        ); // 209 in file

        // 8. Segments Completeness
        assert_eq!(
            pcb.segments.len(),
            409,
            "Exactly 409 segments should be parsed"
        );
        let seg_example = pcb
            .segments
            .iter()
            .find(|s| s.net == 51 && s.start.x == 76.2 && s.start.y == 134.985)
            .expect("Specific segment on net 51 not found");

        assert_eq!(
            seg_example.tstamp, "e258094b-e06f-4066-b280-c2d17c39a2d3",
            "Segment UUID missing or not mapped to tstamp"
        );

        // 9. Footprints Completeness
        assert_eq!(
            pcb.footprints.len(),
            79,
            "Exactly 79 footprints should be parsed"
        );

        // Deep verification on J2 footprint
        let j2 = pcb
            .footprints
            .iter()
            .find(|f| f.property.get("Reference").map(|s| s.as_str()) == Some("J2"))
            .expect("Footprint J2 not found");

        assert_eq!(
            j2.library_link,
            "elektrophon:Jack_3.5mm_WQP-PJ398SM_Vertical"
        );
        assert_eq!(j2.layer, "F.Cu");
        assert_eq!(j2.pos.x, 60.96);
        assert_eq!(j2.pos.y, 60.96);
        assert_eq!(j2.pos.angle, -90.0);
        assert_eq!(j2.descr.as_deref(), Some("TRS 3.5mm, vertical, Thonkiconn, PCB mount, (http://www.qingpu-electronics.com/en/products/WQP-PJ398SM-362.html)"));
        assert!(matches!(j2.footprint_type, FootprintType::ThroughHole));
        assert_eq!(
            j2.path.as_deref(),
            Some("/00000000-0000-0000-0000-00005d64a5b4")
        );

        // Footprint Properties
        assert_eq!(
            j2.property.len(),
            8,
            "J2 should have exactly 8 properties parsed"
        );
        assert_eq!(j2.property.get("Value").map(|s| s.as_str()), Some("IN"));
        assert_eq!(
            j2.property.get("Sim.Pins").map(|s| s.as_str()),
            Some("S=1 T=2 TN=3")
        );

        // Footprint Pads
        assert_eq!(j2.pads.len(), 3, "J2 should have exactly 3 pads");
        let pad_s = j2
            .pads
            .iter()
            .find(|p| p.number == "S")
            .expect("J2 Pad 'S' missing");
        assert_eq!(pad_s.net.ordinal, 1);
        assert_eq!(pad_s.net.name, "GND");
        assert_eq!(pad_s.drill, Some(1.22));
        assert_eq!(pad_s.size, (1.93, 1.83));

        // This will also fail until `Pad::try_from` aliases `uuid` to `tstamp`
        assert_eq!(pad_s.tstamp, None, "tstamp is not empty");
        assert_eq!(
            pad_s.uuid,
            Some("577f61da-6b6e-46e6-9223-763dc9a252fb".to_string()),
            "Pad UUID missing or not mapped to tstamp"
        );

        // Footprint Graphics
        assert_eq!(
            j2.graphic_items.len(),
            29,
            "J2 should have exactly 29 graphic items"
        );

        // 3D Model
        assert_eq!(
            j2.model_3d.as_deref(),
            Some("${KIPRJMOD}/../../../lib/models/PJ301M-12 Thonkiconn v0.2.stp"),
            "J2 3D Model missing"
        );

        // 10. Vias Completeness
        // This will intentionally fail compilation if `vias` isn't added to `Pcb`.
        assert_eq!(pcb.vias.len(), 25, "Exactly 25 vias should be parsed");
        let via1 = &pcb.vias[0];
        assert!((via1.drill - 0.4).abs() < 0.0001);
        assert!((via1.size - 0.8).abs() < 0.0001);
        assert_eq!(via1.layers.0, "F.Cu");
        assert_eq!(via1.layers.1, "B.Cu");

        // 11. Graphic Text (gr_text) Completeness
        // This will intentionally fail compilation if `gr_texts` isn't added to `Pcb`.
        assert_eq!(pcb.gr_texts.len(), 1, "Exactly 1 gr_text should be parsed");
        assert_eq!(pcb.gr_texts[0].text, "summe");
    }
}
