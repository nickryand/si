"use strict";

var _registry = require("../../registry");

// Shared SI Component Properties
_registry.registry.base({
  typeName: "componentSiProperties",
  displayTypeName: "SI Component Internal Properties",
  serviceName: "cea",
  options: function options(c) {
    c.fields.addText({
      name: "integrationId",
      label: "Integration Id",
      options: function options(p) {
        p.readOnly = true;
        p.hidden = true;
        p.required = true;
        p.universal = true;
        p.reference = true;
      }
    });
    c.fields.addText({
      name: "integrationServiceId",
      label: "Integration Service Id",
      options: function options(p) {
        p.readOnly = true;
        p.hidden = true;
        p.required = true;
        p.universal = true;
        p.reference = true;
      }
    });
    c.fields.addNumber({
      name: "version",
      label: "Version",
      options: function options(p) {
        p.numberKind = "int32";
        p.readOnly = true;
        p.hidden = true;
        p.required = true;
        p.universal = true;
      }
    });
  }
});
//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbIi4uLy4uLy4uL3NyYy9jb21wb25lbnRzL3NpLWNlYS9jb21wb25lbnQudHMiXSwibmFtZXMiOlsicmVnaXN0cnkiLCJiYXNlIiwidHlwZU5hbWUiLCJkaXNwbGF5VHlwZU5hbWUiLCJzZXJ2aWNlTmFtZSIsIm9wdGlvbnMiLCJjIiwiZmllbGRzIiwiYWRkVGV4dCIsIm5hbWUiLCJsYWJlbCIsInAiLCJyZWFkT25seSIsImhpZGRlbiIsInJlcXVpcmVkIiwidW5pdmVyc2FsIiwicmVmZXJlbmNlIiwiYWRkTnVtYmVyIiwibnVtYmVyS2luZCJdLCJtYXBwaW5ncyI6Ijs7QUFDQTs7QUFFQTtBQUNBQSxtQkFBU0MsSUFBVCxDQUFjO0FBQ1pDLEVBQUFBLFFBQVEsRUFBRSx1QkFERTtBQUVaQyxFQUFBQSxlQUFlLEVBQUUsa0NBRkw7QUFHWkMsRUFBQUEsV0FBVyxFQUFFLEtBSEQ7QUFJWkMsRUFBQUEsT0FKWSxtQkFJSkMsQ0FKSSxFQUlEO0FBQ1RBLElBQUFBLENBQUMsQ0FBQ0MsTUFBRixDQUFTQyxPQUFULENBQWlCO0FBQ2ZDLE1BQUFBLElBQUksRUFBRSxlQURTO0FBRWZDLE1BQUFBLEtBQUssRUFBRSxnQkFGUTtBQUdmTCxNQUFBQSxPQUhlLG1CQUdQTSxDQUhPLEVBR0o7QUFDVEEsUUFBQUEsQ0FBQyxDQUFDQyxRQUFGLEdBQWEsSUFBYjtBQUNBRCxRQUFBQSxDQUFDLENBQUNFLE1BQUYsR0FBVyxJQUFYO0FBQ0FGLFFBQUFBLENBQUMsQ0FBQ0csUUFBRixHQUFhLElBQWI7QUFDQUgsUUFBQUEsQ0FBQyxDQUFDSSxTQUFGLEdBQWMsSUFBZDtBQUNBSixRQUFBQSxDQUFDLENBQUNLLFNBQUYsR0FBYyxJQUFkO0FBQ0Q7QUFUYyxLQUFqQjtBQVdBVixJQUFBQSxDQUFDLENBQUNDLE1BQUYsQ0FBU0MsT0FBVCxDQUFpQjtBQUNmQyxNQUFBQSxJQUFJLEVBQUUsc0JBRFM7QUFFZkMsTUFBQUEsS0FBSyxFQUFFLHdCQUZRO0FBR2ZMLE1BQUFBLE9BSGUsbUJBR1BNLENBSE8sRUFHSjtBQUNUQSxRQUFBQSxDQUFDLENBQUNDLFFBQUYsR0FBYSxJQUFiO0FBQ0FELFFBQUFBLENBQUMsQ0FBQ0UsTUFBRixHQUFXLElBQVg7QUFDQUYsUUFBQUEsQ0FBQyxDQUFDRyxRQUFGLEdBQWEsSUFBYjtBQUNBSCxRQUFBQSxDQUFDLENBQUNJLFNBQUYsR0FBYyxJQUFkO0FBQ0FKLFFBQUFBLENBQUMsQ0FBQ0ssU0FBRixHQUFjLElBQWQ7QUFDRDtBQVRjLEtBQWpCO0FBV0FWLElBQUFBLENBQUMsQ0FBQ0MsTUFBRixDQUFTVSxTQUFULENBQW1CO0FBQ2pCUixNQUFBQSxJQUFJLEVBQUUsU0FEVztBQUVqQkMsTUFBQUEsS0FBSyxFQUFFLFNBRlU7QUFHakJMLE1BQUFBLE9BSGlCLG1CQUdUTSxDQUhTLEVBR007QUFDckJBLFFBQUFBLENBQUMsQ0FBQ08sVUFBRixHQUFlLE9BQWY7QUFDQVAsUUFBQUEsQ0FBQyxDQUFDQyxRQUFGLEdBQWEsSUFBYjtBQUNBRCxRQUFBQSxDQUFDLENBQUNFLE1BQUYsR0FBVyxJQUFYO0FBQ0FGLFFBQUFBLENBQUMsQ0FBQ0csUUFBRixHQUFhLElBQWI7QUFDQUgsUUFBQUEsQ0FBQyxDQUFDSSxTQUFGLEdBQWMsSUFBZDtBQUNEO0FBVGdCLEtBQW5CO0FBV0Q7QUF0Q1csQ0FBZCIsInNvdXJjZXNDb250ZW50IjpbImltcG9ydCB7IFByb3BOdW1iZXIgfSBmcm9tIFwiLi4vLi4vY29tcG9uZW50cy9wcmVsdWRlXCI7XG5pbXBvcnQgeyByZWdpc3RyeSB9IGZyb20gXCIuLi8uLi9yZWdpc3RyeVwiO1xuXG4vLyBTaGFyZWQgU0kgQ29tcG9uZW50IFByb3BlcnRpZXNcbnJlZ2lzdHJ5LmJhc2Uoe1xuICB0eXBlTmFtZTogXCJjb21wb25lbnRTaVByb3BlcnRpZXNcIixcbiAgZGlzcGxheVR5cGVOYW1lOiBcIlNJIENvbXBvbmVudCBJbnRlcm5hbCBQcm9wZXJ0aWVzXCIsXG4gIHNlcnZpY2VOYW1lOiBcImNlYVwiLFxuICBvcHRpb25zKGMpIHtcbiAgICBjLmZpZWxkcy5hZGRUZXh0KHtcbiAgICAgIG5hbWU6IFwiaW50ZWdyYXRpb25JZFwiLFxuICAgICAgbGFiZWw6IFwiSW50ZWdyYXRpb24gSWRcIixcbiAgICAgIG9wdGlvbnMocCkge1xuICAgICAgICBwLnJlYWRPbmx5ID0gdHJ1ZTtcbiAgICAgICAgcC5oaWRkZW4gPSB0cnVlO1xuICAgICAgICBwLnJlcXVpcmVkID0gdHJ1ZTtcbiAgICAgICAgcC51bml2ZXJzYWwgPSB0cnVlO1xuICAgICAgICBwLnJlZmVyZW5jZSA9IHRydWU7XG4gICAgICB9LFxuICAgIH0pO1xuICAgIGMuZmllbGRzLmFkZFRleHQoe1xuICAgICAgbmFtZTogXCJpbnRlZ3JhdGlvblNlcnZpY2VJZFwiLFxuICAgICAgbGFiZWw6IFwiSW50ZWdyYXRpb24gU2VydmljZSBJZFwiLFxuICAgICAgb3B0aW9ucyhwKSB7XG4gICAgICAgIHAucmVhZE9ubHkgPSB0cnVlO1xuICAgICAgICBwLmhpZGRlbiA9IHRydWU7XG4gICAgICAgIHAucmVxdWlyZWQgPSB0cnVlO1xuICAgICAgICBwLnVuaXZlcnNhbCA9IHRydWU7XG4gICAgICAgIHAucmVmZXJlbmNlID0gdHJ1ZTtcbiAgICAgIH0sXG4gICAgfSk7XG4gICAgYy5maWVsZHMuYWRkTnVtYmVyKHtcbiAgICAgIG5hbWU6IFwidmVyc2lvblwiLFxuICAgICAgbGFiZWw6IFwiVmVyc2lvblwiLFxuICAgICAgb3B0aW9ucyhwOiBQcm9wTnVtYmVyKSB7XG4gICAgICAgIHAubnVtYmVyS2luZCA9IFwiaW50MzJcIjtcbiAgICAgICAgcC5yZWFkT25seSA9IHRydWU7XG4gICAgICAgIHAuaGlkZGVuID0gdHJ1ZTtcbiAgICAgICAgcC5yZXF1aXJlZCA9IHRydWU7XG4gICAgICAgIHAudW5pdmVyc2FsID0gdHJ1ZTtcbiAgICAgIH0sXG4gICAgfSk7XG4gIH0sXG59KTtcbiJdfQ==